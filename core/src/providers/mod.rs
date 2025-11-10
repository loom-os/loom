/// Capability providers for common tools and services
pub mod weather;
pub mod web_search;

pub use weather::WeatherProvider;
pub use web_search::WebSearchProvider;
