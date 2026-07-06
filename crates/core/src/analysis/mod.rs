pub mod conflicts;
pub mod extractor;
pub mod foreshadow;
pub mod item;
pub mod person;
pub mod place;

pub use conflicts::{EyeColorConflictExtractor, RepeatExpressionExtractor};
pub use foreshadow::ForeshadowExtractor;
pub use item::ItemExtractor;
pub use person::PersonExtractor;
pub use place::PlaceExtractor;
