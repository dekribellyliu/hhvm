<?hh

//
// only public, private or protected allowed
//
class A {
  public function __construct(static $a) {}
}

<<__EntryPoint>> function main(): void {}
