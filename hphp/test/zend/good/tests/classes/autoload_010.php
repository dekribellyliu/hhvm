<?hh
function __autoload($name)
{
    echo "In autoload: ";
    var_dump($name);
}

class C implements UndefI
{
}

<<__EntryPoint>> function main(): void {}
