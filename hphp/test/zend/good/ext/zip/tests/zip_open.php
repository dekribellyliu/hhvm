<?hh <<__EntryPoint>> function main(): void {
$zip = zip_open(dirname(__FILE__)."/test_procedural.zip");

echo is_resource($zip) ? "OK" : "Failure";
}
