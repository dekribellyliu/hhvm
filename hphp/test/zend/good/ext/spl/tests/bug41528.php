<?hh
class ClassOne extends ArrayObject
{
    public $a = 2;
}
<<__EntryPoint>> function main(): void {
$classOne    = new ClassOne();
$classOne->a = 1;

var_dump($classOne);
var_dump($classOne->a);

$classOne = unserialize(serialize($classOne));

var_dump($classOne);
var_dump($classOne->a);
}
