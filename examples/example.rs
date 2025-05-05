use toors::ToolCollection;
use toors_derive;

struct Collection {}

impl Collection {
    #[tool]
    /// This function does nothing
    fn noop() {}

    #[tool]
    /// This function takes some arguments
    fn with_args(args: i64) {
        args;
    }
}

#[tool]
/// Gets weather from lat and long
fn get_weather(_lat: &str, _long: &str) {}

fn main() {
    let mut col = ToolCollection::default();

    col.register(get_weather);
}

