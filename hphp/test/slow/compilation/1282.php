<?php

class A {
 function test(A $a) {
 $a->foo();
}
 function foo() {
 print 'foo';
}
}
<<__EntryPoint>> function main() {}
