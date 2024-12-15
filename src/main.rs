//! ## Task Description
//!
//! The goal is to develop a backend service for shortening URLs using CQRS
//! (Command Query Responsibility Segregation) and ES (Event Sourcing)
//! approaches. The service should support the following features:
//!
//! ## Functional Requirements
//!
//! ### Creating a short link with a random slug
//!
//! The user sends a long URL, and the service returns a shortened URL with a
//! random slug.
//!
//! ### Creating a short link with a predefined slug
//!
//! The user sends a long URL along with a predefined slug, and the service
//! checks if the slug is unique. If it is unique, the service creates the short
//! link.
//!
//! ### Counting the number of redirects for the link
//!
//! - Every time a user accesses the short link, the click count should
//!   increment.
//! - The click count can be retrieved via an API.
//!
//! ### CQRS+ES Architecture
//!
//! CQRS: Commands (creating links, updating click count) are separated from
//! queries (retrieving link information).
//!
//! Event Sourcing: All state changes (link creation, click count update) must be
//! recorded as events, which can be replayed to reconstruct the system's state.
//!
//! ### Technical Requirements
//!
//! - The service must be built using CQRS and Event Sourcing approaches.
//! - The service must be possible to run in Rust Playground (so no database like
//!   Postgres is allowed)
//! - Public API already written for this task must not be changed (any change to
//!   the public API items must be considered as breaking change).
//! - Event Sourcing should be actively utilized for implementing logic, rather
//!   than existing without a clear purpose.

#![allow(unused_variables, dead_code)]
use crate::commands::CommandHandler;
use queries::QueryHandler;

use std::collections::HashMap;
// better to use rand crate. But or any method to generate unique hash
use std::time::{SystemTime, UNIX_EPOCH};

/// All possible errors of the [`UrlShortenerService`].
#[derive(Debug, PartialEq)]
pub enum ShortenerError {
    /// This error occurs when an invalid [`Url`] is provided for shortening.
    InvalidUrl,

    /// This error occurs when an attempt is made to use a slug (custom alias)
    /// that already exists.
    SlugAlreadyInUse,

    /// This error occurs when the provided [`Slug`] does not map to any existing
    /// short link.
    SlugNotFound,
}

/// A unique string (or alias) that represents the shortened version of the
/// URL.
#[derive(Eq, Clone, Debug, Hash, PartialEq)]
pub struct Slug(pub String);

/// The original URL that the short link points to.
#[derive(Clone, Debug, PartialEq)]
pub struct Url(pub String);

/// Shortened URL representation.
#[derive(Debug, Clone, PartialEq)]
pub struct ShortLink {
    /// A unique string (or alias) that represents the shortened version of the
    /// URL.
    pub slug: Slug,

    /// The original URL that the short link points to.
    pub url: Url,
}

/// Statistics of the [`ShortLink`].
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    /// [`ShortLink`] to which this [`Stats`] are related.
    pub link: ShortLink,

    /// Count of redirects of the [`ShortLink`].
    pub redirects: u64,
}

/// Commands for CQRS.
pub mod commands {
    use super::{ShortLink, ShortenerError, Slug, Url};

    /// Trait for command handlers.
    pub trait CommandHandler {
        /// Creates a new short link. It accepts the original url and an
        /// optional [`Slug`]. If a [`Slug`] is not provided, the service will generate
        /// one. Returns the newly created [`ShortLink`].
        ///
        /// ## Errors
        ///
        /// See [`ShortenerError`].
        fn handle_create_short_link(
            &mut self,
            url: Url,
            slug: Option<Slug>,
        ) -> Result<ShortLink, ShortenerError>;

        /// Processes a redirection by [`Slug`], returning the associated
        /// [`ShortLink`] or a [`ShortenerError`].
        fn handle_redirect(&mut self, slug: Slug) -> Result<ShortLink, ShortenerError>;
    }
}

/// Queries for CQRS
pub mod queries {
    use super::{ShortenerError, Slug, Stats};

    /// Trait for query handlers.
    pub trait QueryHandler {
        /// Returns the [`Stats`] for a specific [`ShortLink`], such as the
        /// number of redirects (clicks).
        ///
        /// [`ShortLink`]: super::ShortLink
        fn get_stats(&self, slug: Slug) -> Result<Stats, ShortenerError>;
    }
}

/// Events for Event Sourcing
#[derive(Debug)]
pub enum Event {
    LinkCreated(ShortLink),
    LinkRedirected(Slug),
}

/// CQRS and Event Sourcing-based service implementation
pub struct UrlShortenerService {
    links: HashMap<Slug, ShortLink>,
    stats: HashMap<Slug, Stats>,
    event_store: Vec<Event>,
}

impl UrlShortenerService {
    /// Creates a new instance of the service
    pub fn new() -> Self {
        Self {
            links: HashMap::new(),
            stats: HashMap::new(),
            event_store: Vec::new(),
        }
    }
}

impl commands::CommandHandler for UrlShortenerService {
    fn handle_create_short_link(
        &mut self,
        url: Url,
        slug: Option<Slug>,
    ) -> Result<ShortLink, ShortenerError> {
        let slug = match slug {
            Some(s) => {
                if self.links.contains_key(&s) {
                    return Err(ShortenerError::SlugAlreadyInUse);
                }
                s
            }
            None => {
                let random_slug = Slug(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .subsec_nanos()
                        .to_string(),
                );
                while self.links.contains_key(&random_slug) {
                    let random_slug = Slug(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .subsec_nanos()
                            .to_string(),
                    );
                }
                random_slug
            }
        };

        let short_link = ShortLink {
            slug: slug.clone(),
            url,
        };
        self.links.insert(slug.clone(), short_link.clone());
        self.stats.insert(
            slug.clone(),
            Stats {
                link: short_link.clone(),
                redirects: 0,
            },
        );
        self.event_store
            .push(Event::LinkCreated(short_link.clone()));

        Ok(short_link)
    }

    fn handle_redirect(&mut self, slug: Slug) -> Result<ShortLink, ShortenerError> {
        if let Some(short_link) = self.links.get(&slug) {
            if let Some(stat) = self.stats.get_mut(&slug) {
                stat.redirects += 1;
                self.event_store.push(Event::LinkRedirected(slug.clone()));
            }
            Ok(short_link.clone())
        } else {
            Err(ShortenerError::SlugNotFound)
        }
    }
}

impl queries::QueryHandler for UrlShortenerService {
    fn get_stats(&self, slug: Slug) -> Result<Stats, ShortenerError> {
        self.stats
            .get(&slug)
            .cloned()
            .ok_or(ShortenerError::SlugNotFound)
    }
}

fn main() {
    let mut service = UrlShortenerService::new();

    // Example usage
    let url = Url("https://example.com".to_string());
    let slug = service.handle_create_short_link(url.clone(), None).unwrap();
    println!("Created short link: {:?}", slug);

    // Redirecting the short link
    let redirected_link = service.handle_redirect(slug.slug.clone()).unwrap();
    let redirected_link = service.handle_redirect(slug.slug.clone()).unwrap();
    println!("Redirected to: {:?}", redirected_link);

    // Getting stats
    let stats = service.get_stats(slug.slug.clone()).unwrap();
    println!("Stats: {:?}", stats);
}
