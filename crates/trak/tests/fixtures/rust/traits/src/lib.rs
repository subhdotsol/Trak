trait Greet {
    fn greet(&self) -> String;
}

struct Person {
    name: String,
}

impl Greet for Person {
    fn greet(&self) -> String {
        self.name.clone()
    }
}

fn greet_all(people: &[Person]) {
    for person in people {
        let _ = person.greet();
    }
}