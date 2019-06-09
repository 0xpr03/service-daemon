#[derive(Fail, Debug)]
pub enum DBError {
    #[fail(display = "Failed to load services from data, services already loaded!")]
    ServicesNotEmpty,
    #[fail(display = "Invalid instance ID: {}", _0)]
    InvalidInstance(usize),
    #[fail(display = "Unable to start, IO error: {}", _0)]
    StartupIOError(::std::io::Error),
}

pub struct DB {
    pub foo: String,
}