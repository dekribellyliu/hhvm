<?hh
/* Prototype  : array array_unique(array $input)
 * Description: Removes duplicate values from array
 * Source code: ext/standard/array.c
*/

/*
 * Testing the functionality of array_unique() by passing different
 * associative arrays having different values to $input argument.
*/

// get a class
class classA
{
  public function __toString() {
     return "Class A object";
  }
}
<<__EntryPoint>> function main(): void {
echo "*** Testing array_unique() : assoc. array with diff. values to \$input argument ***\n";

// get an unset variable
$unset_var = 10;
unset ($unset_var);

// get a resource variable
$fp = fopen(__FILE__, "r");

// get a heredoc string
$heredoc = <<<EOT
Hello world
EOT;

// associative arrays with different values
$inputs = array (
       // arrays with integer values
/*1*/  array('0' => 0, '1' => 0),
       array("one" => 1, 'two' => 2, "three" => 1, 4 => 1),

       // arrays with float values
/*3*/  array("float1" => 2.3333, "float2" => 2.3333),
       array("f1" => 1.2, 'f2' => 3.33, 3 => 4.89999922839999, 'f4' => 1.2),

       // arrays with string values
/*5*/  array(111 => "\tHello", "red" => "col\tor", 2 => "\v\fworld", 3.3 =>  "\tHello"),
       array(111 => '\tHello', "red" => 'col\tor', 2 => '\v\fworld', 3.3 =>  '\tHello'),
       array(1 => "hello", "heredoc" => $heredoc, $heredoc),

       // array with object, unset variable and resource variable
/*8*/ array(11 => new classA(), "unset" => @$unset_var, "resource" => $fp, new classA(), $fp),
);

// loop through each sub-array of $inputs to check the behavior of array_unique()
$iterator = 1;
foreach($inputs as $input) {
  echo "-- Iteration $iterator --\n";
  var_dump( array_unique($input) );
  $iterator++;
}

fclose($fp);

echo "Done";
}
