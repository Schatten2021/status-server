#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct Status<T> {
    pub status: T,
}
impl<T> Status<T> {
    pub const fn new(val: T) -> Self {
        Self { status: val }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub struct Notification<T> {
    #[serde(alias="notifications", alias="notify")]
    pub notification: T,
}
impl<T> Notification<T> {
    pub const fn new(val: T) -> Self {
        Self { notification: val }
    }
}
