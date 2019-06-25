<?hh

class ParentClass { }

class ChildClass extends ParentClass {
    public function testIsCallable() {
        var_dump(is_callable(array($this, 'parent::testIsCallable')));
    }
    public function testIsCallable2() {
        var_dump(is_callable(array($this, 'static::testIsCallable2')));
    }
}
<<__EntryPoint>> function main(): void {
$child = new ChildClass();
$child->testIsCallable();
$child->testIsCallable2();
}
