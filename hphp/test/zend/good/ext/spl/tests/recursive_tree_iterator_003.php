<?hh <<__EntryPoint>> function main(): void {
try {
    new RecursiveTreeIterator(new ArrayIterator(array()));
} catch (InvalidArgumentException $e) {
    echo "InvalidArgumentException thrown\n";
}
echo "===DONE===\n";
}
