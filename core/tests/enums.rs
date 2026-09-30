named_enum! {
    /// A test enum.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Fruit {
        /// First.
        Apple => "apple",
        /// Second.
        Pear => "pear",
        /// Third.
        Plum => "plum",
    }
}

#[test]
fn generated_items_follow_the_declaration() {
    assert_eq!(Fruit::ALL, [Fruit::Apple, Fruit::Pear, Fruit::Plum]);
    assert_eq!(Fruit::Pear.position(), 1);
    assert_eq!(Fruit::Plum.next(), Fruit::Apple, "wraps around");
    assert_eq!(Fruit::names("|"), "apple|pear|plum");
}

#[test]
fn parse_is_the_inverse_of_as_str() {
    for fruit in Fruit::ALL {
        assert_eq!(Fruit::parse(fruit.as_str()), Some(*fruit));
    }
    assert_eq!(Fruit::parse(" pear "), Some(Fruit::Pear));
    assert_eq!(Fruit::parse("banana"), None);
}
