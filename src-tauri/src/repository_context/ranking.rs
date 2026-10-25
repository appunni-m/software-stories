//! Bounded candidate collection shared by lexical, identifier and declaration search.

use super::*;
use tantivy::{
    Order,
    collector::{
        Count, TopDocs,
        sort_key::{SortBySimilarityScore, SortByStaticFastValue, SortByString},
    },
};

type CandidateKey = (
    f32,
    (
        Option<String>,
        (Option<u64>, (Option<u64>, Option<String>)),
    ),
);

pub(super) fn candidates(
    searcher: &tantivy::Searcher,
    query: &dyn Query,
) -> tantivy::Result<(Vec<(f32, tantivy::DocAddress)>, usize)> {
    candidates_with_limit(searcher, query, MAX_CANDIDATES)
}

pub(super) fn candidates_with_limit(
    searcher: &tantivy::Searcher,
    query: &dyn Query,
    limit: usize,
) -> tantivy::Result<(Vec<(f32, tantivy::DocAddress)>, usize)> {
    // Stable range identity participates before TopDocs caps the results.
    // Sorting retained hits later cannot recover ranges discarded at the cap.
    let collector = TopDocs::with_limit(limit).order_by((
        (SortBySimilarityScore, Order::Desc),
        (
            (SortByString::for_field("source_path"), Order::Asc),
            (
                (
                    SortByStaticFastValue::<u64>::for_field("start_byte"),
                    Order::Asc,
                ),
                (
                    (
                        SortByStaticFastValue::<u64>::for_field("end_byte"),
                        Order::Asc,
                    ),
                    (SortByString::for_field("range_key"), Order::Asc),
                ),
            ),
        ),
    ));
    let (hits, count): (Vec<(CandidateKey, tantivy::DocAddress)>, usize) =
        searcher.search(query, &(collector, Count))?;
    let hits = hits
        .into_iter()
        .map(|((score, (path, (start, (end, key)))), address)| {
            if path.is_none()
                || start.is_none()
                || end.is_none()
                || key.is_none()
                || !score.is_finite()
            {
                return Err(tantivy::TantivyError::InvalidArgument(
                    "The code search ordering fields are invalid.".into(),
                ));
            }
            Ok((score, address))
        })
        .collect::<tantivy::Result<_>>()?;
    Ok((hits, count))
}

#[cfg(test)]
mod tests;
