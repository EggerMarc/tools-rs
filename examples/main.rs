use toors::toors_derive;
use toors::ToolCollection;

static COLLECTION = ToolCollection::default();
#[toors(collections=[])]
/// This function doesn't do anything
fn noop() {}

fn main() {
    let mut col = ToolCollection::default();

}
