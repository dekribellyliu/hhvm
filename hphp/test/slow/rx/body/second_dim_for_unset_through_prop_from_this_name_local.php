<?hh

class C {
  public $p;

  <<__Rx>>
  public function bad() {
    $p1 = new stdClass();
    $p1->q = array(3 => true);
    $this->p = $p1;

    $lq = 'q';
    unset($this->p->{$lq}[3]);
  }
}

<<__EntryPoint>>
function test() {
  $c = new C();
  $c->bad();
}
