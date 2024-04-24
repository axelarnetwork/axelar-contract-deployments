/// Trait for archiving and retrieving tracer data used by a `Transport`.
///
/// This trait is generic over the `Data` type, which represents the actual data
/// used to seed the listening agent and to be persisted by the delivering
/// agent. It is expected to be used as save point.
#[trait_variant::make(Send)]
pub trait State<Data: Send> {
    /// The expected error to be fould while saving and loading the `Data` type.
    type Error: std::error::Error + Send + 'static;

    /// Retrieves the latest valid Data from storage.
    async fn get(&self) -> Result<Option<Data>, Self::Error>;

    /// Saves the latest valid Data to storage.
    async fn set(&self, message: Data) -> Result<(), Self::Error>;
}
