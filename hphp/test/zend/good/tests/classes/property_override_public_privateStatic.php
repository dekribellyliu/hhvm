<?hh
class A
{
    public $p = "A::p";
    function showA()
    {
        echo $this->p . "\n";
    }
}

class B extends A
{
    private static $p = "B::p (static)";
    static function showB()
    {
        echo self::$p . "\n";
    }
}

<<__EntryPoint>> function main(): void {
$a = new A;
$a->showA();

$b = new B;
$b->showA();
B::showB();
}
