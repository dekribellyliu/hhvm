<?hh
/*
 *  proto int preg_match(string pattern, string subject [, array subpatterns [, int flags [, int offset]]])
 * Function is implemented in ext/pcre/php_pcre.c */
<<__EntryPoint>> function main(): void {
$string = 'Hello, world. [*], this is \ a string';
var_dump(preg_match('/^[hH]ello,\s/', $string, &$match1)); //finds "Hello, "
var_dump($match1);
var_dump(preg_match('/l^o,\s\w{5}/', $string, &$match2, PREG_OFFSET_CAPTURE)); // tries to find "lo, world" at start of string
var_dump($match2);
var_dump(preg_match('/\[\*\],\s(.*)/', $string, &$match3)); //finds "[*], this is \ a string";
var_dump($match3);
var_dump(preg_match('@\w{4}\s\w{2}\s\\\(?:\s.*)@', $string, &$match4, PREG_OFFSET_CAPTURE, 14)); //finds "this is \ a string" (with non-capturing parentheses)
var_dump($match4);
var_dump(preg_match('/hello world/', $string, &$match5)); //tries to find "hello world" (should be Hello, world)
var_dump($match5);
}
