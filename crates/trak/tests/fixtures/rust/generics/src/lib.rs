struct Pair<A, B> {
    first: A,
    second: B,
}

fn swap<A, B>(pair: Pair<A, B>) -> Pair<B, A> {
    Pair {
        first: pair.second,
        second: pair.first,
    }
}

fn use_pair() -> Pair<i32, &'static str> {
    swap(Pair {
        first: 1,
        second: "one",
    })
}