<?hh

class C {
  public $val;
  public function __debugInfo() {
    return $this->val;
  }
  public function __construct($val) {
    $this->val = $val;
  }
}
<<__EntryPoint>> function main(): void {
$c = new C(new stdClass);
var_dump($c);
}
