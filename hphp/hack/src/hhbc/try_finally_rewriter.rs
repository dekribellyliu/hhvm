// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the "hack" directory of this source tree.

#![allow(dead_code)]

use crate::emit_statement::{LazyState, Level};
use crate::reified_generics_helpers as reified;

use ast_scope_rust as ast_scope;
use emit_expression_rust as emit_expression;
use emit_fatal_rust as emit_fatal;
use emit_pos_rust::emit_pos;
use env::{emitter::Emitter, iterator::Iter, jump_targets as jt, Env};
use hhbc_ast_rust::{self as hhbc_ast, Instruct};
use instruction_sequence_rust::{InstrSeq, Result};
use label::Label;
use label_rust as label;
use local_rust as local;
use oxidized::pos::Pos;

use bitflags::bitflags;

use std::{borrow::Cow, collections::BTreeMap};

type LabelMap<'a> = BTreeMap<label::Id, &'a Instruct>;

pub(super) struct JumpInstructions<'a>(LabelMap<'a>);
impl JumpInstructions<'_> {
    /// Collects list of Ret* and non rewritten Break/Continue instructions inside try body.
    pub(super) fn collect<'a>(is: &'a InstrSeq, jt_gen: &mut jt::Gen) -> JumpInstructions<'a> {
        fn get_label_id(jt_gen: &mut jt::Gen, is_break: bool, level: Level) -> label::Id {
            use jt::ResolvedJumpTarget::*;
            match jt_gen.jump_targets().get_target_for_level(is_break, level) {
                ResolvedRegular(target_label, _)
                | ResolvedTryFinally(jt::ResolvedTryFinally { target_label, .. }) => {
                    jt_gen.get_id_for_label(target_label)
                }
                _ => panic!("impossible"),
            }
        }
        JumpInstructions(is.fold_left(
            &mut |mut acc: LabelMap<'a>, i| {
                use hhbc_ast::Instruct::*;
                use hhbc_ast::InstructControlFlow::{RetC, RetCSuspended, RetM};
                use hhbc_ast::InstructSpecialFlow::{Break, Continue, Goto};
                match i {
                    &ISpecialFlow(Break(level)) => {
                        acc.insert(get_label_id(jt_gen, true, level as Level), i);
                    }
                    &ISpecialFlow(Continue(level)) => {
                        acc.insert(get_label_id(jt_gen, false, level as Level), i);
                    }
                    &IContFlow(ref cont_flow) => match cont_flow {
                        RetC | RetCSuspended | RetM(_) => {
                            acc.insert(jt_gen.get_id_for_return(), i);
                        }
                        _ => (),
                    },
                    &ISpecialFlow(Goto(ref l)) => {
                        acc.insert(jt_gen.get_id_for_label(Label::Named(l.clone())), i);
                    }
                    _ => (),
                };
                acc
            },
            LabelMap::new(),
        ))
    }
}

/// Delete Ret*, Break/Continue/Jmp(Named) instructions from the try body
pub(super) fn cleanup_try_body(is: &InstrSeq) -> InstrSeq {
    use hhbc_ast::Instruct::*;
    use hhbc_ast::InstructControlFlow::{RetC, RetCSuspended, RetM};
    is.filter_map(&mut |i| match i {
        &ISpecialFlow(_) => None,
        &IContFlow(ref cont_flow) => match cont_flow {
            RetC | RetCSuspended | RetM(_) => None,
            _ => Some(i.clone()),
        },
        _ => Some(i.clone()),
    })
}

pub(super) fn emit_jump_to_label(l: Label, iters: Vec<Iter>) -> InstrSeq {
    if iters.is_empty() {
        InstrSeq::make_jmp(l)
    } else {
        InstrSeq::make_iter_break(l, iters.iter().map(|i| i.next).collect())
    }
}

pub(super) fn emit_save_label_id(local_gen: &mut local::Gen, id: usize) -> InstrSeq {
    InstrSeq::gather(vec![
        InstrSeq::make_int(id as isize),
        InstrSeq::make_setl(local_gen.get_label().clone()),
        InstrSeq::make_popc(),
    ])
}

fn get_pos_for_error<'a>(env: &'a Env<'a>) -> Cow<'a, Pos> {
    for item in env.scope.iter() {
        use ast_scope::ScopeItem;
        match item {
            ScopeItem::Function(fd) => return Pos::first_char_of_line(&fd.span),
            // For methods, it points to class not the method.. weird
            ScopeItem::Class(cd) => return Pos::first_char_of_line(&cd.span),
            ScopeItem::Method(_) | ScopeItem::Lambda(_) | ScopeItem::LongLambda(_) => (),
        }
    }
    Cow::Owned(Pos::make_none())
}

pub fn emit_goto(
    in_finally_epilogue: bool,
    label: String,
    env: &mut Env,
    local_gen: &mut local::Gen,
) -> Result {
    let in_using_opt = env
        .jump_targets_gen
        .get_labels_in_function()
        .get(&label)
        .copied();
    match in_using_opt {
        None => Err(emit_fatal::raise_fatal_parse(
            &get_pos_for_error(env),
            format!("'goto' to undefined label '{}'", label),
        )),
        Some(in_using) => {
            // CONSIDER: we don't need to assign state id for label
            // for cases when it is not necessary, i.e. when jump target is in the same
            // scope. HHVM does not do this today, do the same for compatibility reasons
            let jt_gen = &mut env.jump_targets_gen;
            match jt_gen.jump_targets().find_goto_target(&label) {
                jt::ResolvedGotoTarget::Label(iters) => {
                    let preamble = if !in_finally_epilogue {
                        InstrSeq::make_empty()
                    } else {
                        InstrSeq::make_unsetl(local_gen.get_label().clone())
                    };
                    Ok(InstrSeq::gather(vec![
                        preamble,
                        emit_jump_to_label(Label::Named(label), iters),
                    ]))
                }
                jt::ResolvedGotoTarget::Finally(jt::ResolvedGotoFinally {
                    rgf_finally_start_label,
                    rgf_iterators_to_release,
                }) => {
                    let preamble = if in_finally_epilogue {
                        InstrSeq::make_empty()
                    } else {
                        let label_id = jt_gen.get_id_for_label(Label::Named(label.clone()));
                        emit_save_label_id(local_gen, label_id)
                    };
                    Ok(InstrSeq::gather(vec![
                        preamble,
                        emit_jump_to_label(rgf_finally_start_label, rgf_iterators_to_release),
                        // emit goto as an indicator for try/finally rewriter to generate
                        // finally epilogue, try/finally rewriter will remove it.
                        InstrSeq::make_goto(label),
                    ]))
                }
                jt::ResolvedGotoTarget::GotoFromFinally => Err(emit_fatal::raise_fatal_parse(
                    &get_pos_for_error(env),
                    "Goto to a label outside a finally block is not supported",
                )),
                jt::ResolvedGotoTarget::GotoInvalidLabel => Err(emit_fatal::raise_fatal_parse(
                    &get_pos_for_error(env),
                    if in_using {
                        "'goto' into or across using statement is disallowed"
                    } else {
                        "'goto' into loop or switch statement is disallowed"
                    },
                )),
            }
        }
    }
}

pub(super) fn emit_return(e: &mut Emitter, in_finally_epilogue: bool, env: &mut Env) -> InstrSeq {
    // check if there are try/finally region
    let jt_gen = &env.jump_targets_gen;
    match jt_gen.jump_targets().get_closest_enclosing_finally_label() {
        None => {
            // no finally blocks, but there might be some iterators that should be
            // released before exit - do it
            let ctx = e.emit_state();
            let num_out = ctx.num_out;
            let verify_out = ctx.verify_out.clone();
            let verify_return = ctx.verify_return.clone();
            let release_iterators_instr = InstrSeq::gather(
                jt_gen
                    .jump_targets()
                    .iterators()
                    .map(|i| InstrSeq::make_iterfree(i.next))
                    .collect(),
            );
            let mut instrs = Vec::with_capacity(5);
            if in_finally_epilogue {
                let load_retval_instr =
                    InstrSeq::make_cgetl(e.local_gen_mut().get_retval().clone());
                instrs.push(load_retval_instr);
            }
            let verify_return_instr = verify_return.map_or_else(
                || InstrSeq::make_empty(),
                |h| {
                    use reified::ReificationLevel;
                    let h = reified::convert_awaitable(env, h.clone());
                    let h = reified::remove_erased_generics(env, h);
                    match reified::has_reified_type_constraint(env, &h) {
                        ReificationLevel::Unconstrained => InstrSeq::make_empty(),
                        ReificationLevel::Not => InstrSeq::make_verify_ret_type_c(),
                        ReificationLevel::Maybe => InstrSeq::gather(vec![
                            emit_expression::get_type_structure_for_hint(
                                e.options(),
                                &[],
                                &BTreeMap::new(),
                                h,
                            ),
                            InstrSeq::make_verify_ret_type_ts(),
                        ]),
                        ReificationLevel::Definitely => {
                            let check = InstrSeq::gather(vec![
                                InstrSeq::make_dup(),
                                InstrSeq::make_istypec(hhbc_ast::IstypeOp::OpNull),
                            ]);
                            reified::simplify_verify_type(
                                env,
                                &Pos::make_none(),
                                check,
                                &h,
                                InstrSeq::make_verify_ret_type_ts(),
                                e.label_gen_mut(),
                            )
                        }
                    }
                },
            );
            instrs.extend(vec![
                verify_return_instr,
                verify_out,
                release_iterators_instr,
                if num_out != 0 {
                    InstrSeq::make_retm(num_out + 1)
                } else {
                    InstrSeq::make_retc()
                },
            ]);
            InstrSeq::gather(instrs)
        }
        // ret is in finally block and there might be iterators to release -
        // jump to finally block via Jmp
        Some((target_label, iterators_to_release)) => {
            let preamble = if in_finally_epilogue {
                InstrSeq::make_empty()
            } else {
                let jt_gen = &mut env.jump_targets_gen;
                let save_state = emit_save_label_id(e.local_gen_mut(), jt_gen.get_id_for_return());
                let save_retval = InstrSeq::gather(vec![
                    InstrSeq::make_setl(e.local_gen_mut().get_retval().clone()),
                    InstrSeq::make_popc(),
                ]);
                InstrSeq::gather(vec![save_state, save_retval])
            };
            InstrSeq::gather(vec![
                preamble,
                emit_jump_to_label(target_label, iterators_to_release),
                // emit ret instr as an indicator for try/finally rewriter to generate
                // finally epilogue, try/finally rewriter will remove it.
                InstrSeq::make_retc(),
            ])
        }
    }
}

bitflags! {
    pub(super) struct EmitBreakOrContinueFlags: u8 {
        const IS_BREAK =            0b01;
        const IN_FINALLY_EPILOGUE = 0b10;
    }
}

pub(super) fn emit_break_or_continue(
    e: &mut Emitter,
    flags: EmitBreakOrContinueFlags,
    env: &mut Env,
    pos: &Pos,
    level: Level,
) -> InstrSeq {
    let jt_gen = &mut env.jump_targets_gen;
    let in_finally_epilogue = flags.contains(EmitBreakOrContinueFlags::IN_FINALLY_EPILOGUE);
    let is_break = flags.contains(EmitBreakOrContinueFlags::IS_BREAK);
    match jt_gen.jump_targets().get_target_for_level(is_break, level) {
        jt::ResolvedJumpTarget::NotFound => {
            emit_fatal::emit_fatal_for_break_continue(e, pos, level)
        }
        jt::ResolvedJumpTarget::ResolvedRegular(target_label, iterators_to_release) => {
            let preamble = if in_finally_epilogue && level == 1 {
                InstrSeq::make_unsetl(e.local_gen_mut().get_label().clone())
            } else {
                InstrSeq::make_empty()
            };
            InstrSeq::gather(vec![
                preamble,
                emit_pos(e, pos),
                emit_jump_to_label(target_label, iterators_to_release),
            ])
        }
        jt::ResolvedJumpTarget::ResolvedTryFinally(jt::ResolvedTryFinally {
            target_label,
            finally_label,
            iterators_to_release,
            adjusted_level,
        }) => {
            let preamble = if !in_finally_epilogue {
                let label_id = jt_gen.get_id_for_label(target_label.clone());
                emit_save_label_id(e.local_gen_mut(), label_id)
            } else {
                InstrSeq::make_empty()
            };
            let adjusted_level = adjusted_level as isize;
            InstrSeq::gather(vec![
                preamble,
                emit_jump_to_label(finally_label, iterators_to_release),
                emit_pos(e, pos),
                // emit break/continue instr as an indicator for try/finally rewriter
                // to generate finally epilogue - try/finally rewriter will remove it.
                if is_break {
                    InstrSeq::make_break(adjusted_level)
                } else {
                    InstrSeq::make_continue(adjusted_level)
                },
            ])
        }
    }
}

fn emit_finally_epilogue(
    e: &mut Emitter,
    env: &mut Env,
    pos: &Pos,
    jump_instrs: JumpInstructions,
    finally_end: Label,
) -> Result {
    fn emit_instr(e: &mut Emitter, env: &mut Env, pos: &Pos, i: &Instruct) -> Result {
        use hhbc_ast::Instruct::*;
        use hhbc_ast::InstructControlFlow::{RetC, RetCSuspended, RetM};
        use hhbc_ast::InstructSpecialFlow::{Break, Continue, Goto};
        let fail = || {
            panic!("unexpected instruction: only Ret* or Break/Continue/Jmp(Named) are expected")
        };
        match i {
            &IContFlow(ref cont_flow) => match cont_flow {
                RetC | RetCSuspended | RetM(_) => Ok(emit_return(e, true, env)),
                _ => fail(),
            },
            &ISpecialFlow(Break(level)) => Ok(emit_break_or_continue(
                e,
                EmitBreakOrContinueFlags::IS_BREAK | EmitBreakOrContinueFlags::IN_FINALLY_EPILOGUE,
                env,
                pos,
                level as Level,
            )),
            &ISpecialFlow(Continue(level)) => Ok(emit_break_or_continue(
                e,
                EmitBreakOrContinueFlags::IN_FINALLY_EPILOGUE,
                env,
                pos,
                level as Level,
            )),
            &ISpecialFlow(Goto(ref label)) => {
                emit_goto(true, label.clone(), env, e.local_gen_mut())
            }
            _ => fail(),
        }
    };
    Ok(if jump_instrs.0.is_empty() {
        InstrSeq::make_empty()
    } else if jump_instrs.0.len() == 1 {
        let (_, instr) = jump_instrs.0.iter().next().unwrap();
        InstrSeq::gather(vec![
            emit_pos(e, pos),
            InstrSeq::make_issetl(e.local_gen_mut().get_label().clone()),
            InstrSeq::make_jmpz(finally_end),
            emit_instr(e, env, pos, instr)?,
        ])
    } else {
        // mimic HHVM behavior:
        // in some cases ids can be non-consequtive - this might happen i.e. return statement
        //  appear in the block and it was assigned a high id before.
        //  ((3, Return), (1, Break), (0, Continue))
        //  In thid case generate switch as
        //  switch  (id) {
        //     L0 -> handler for continue
        //     L1 -> handler for break
        //     FinallyEnd -> empty
        //     L3 -> handler for return
        //  }
        //
        // This function builds a list of labels and jump targets for switch.
        // It is possible that cases ids are not consequtive
        // [L1,L2,L4]. Vector of labels in switch should be dense so we need to
        // fill holes with a label that points to the end of finally block
        // [End, L1, L2, End, L4]
        let (max_id, _) = jump_instrs.0.iter().next_back().unwrap();
        let (mut labels, mut bodies) = (vec![], vec![]);
        let mut n: isize = *max_id as isize;
        // lst is already sorted - BTreeMap/IMap bindings took care of it
        // TODO: add is_sorted assert to make sure this behavior is preserved for labels
        for (id, instr) in jump_instrs.0.into_iter().rev() {
            let mut done = false;
            while !done {
                // NOTE(hrust) looping is equivalent to recursing without consuming instr
                done = (id as isize) == n;
                let (label, body) = if done {
                    let label = e.label_gen_mut().next_regular();
                    let body = InstrSeq::gather(vec![
                        InstrSeq::make_label(label.clone()),
                        emit_instr(e, env, pos, instr)?,
                    ]);
                    (label, body)
                } else {
                    (finally_end.clone(), InstrSeq::make_empty())
                };
                labels.push(label);
                bodies.push(body);
                n -= 1;
            }
        }
        // NOTE(hrust): base case when empty and n >= 0
        for _ in 0..=n {
            labels.push(finally_end.clone());
            bodies.push(InstrSeq::make_empty());
        }
        InstrSeq::gather(vec![
            emit_pos(e, pos),
            InstrSeq::make_issetl(e.local_gen_mut().get_label().clone()),
            InstrSeq::make_jmpz(finally_end),
            InstrSeq::make_cgetl(e.local_gen_mut().get_label().clone()),
            InstrSeq::make_switch(labels),
            InstrSeq::gather(bodies),
        ])
    })
}

// TODO: This codegen is unnecessarily complex.  Basically we are generating
//
// IsSetL temp
// JmpZ   finally_end
// CGetL  temp
// Switch Unbounded 0 <L4 L5>
// L5:
// UnsetL temp
// Jmp LContinue
// L4:
// UnsetL temp
// Jmp LBreak
//
// Two problems immediately come to mind. First, why is the unset in every case,
// instead of after the CGetL?  Surely the unset doesn't modify the stack.
// Second, now we have a jump-to-jump situation.
//
// Would it not make more sense to instead say
//
// IsSetL temp
// JmpZ   finally_end
// CGetL  temp
// UnsetL temp
// Switch Unbounded 0 <LBreak LContinue>
//
// ?
