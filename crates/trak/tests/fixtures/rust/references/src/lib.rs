struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn origin() -> Self {
        Point { x: 0, y: 0 }
    }
}

fn area(p: &Point) -> i32 {
    p.x * p.y
}