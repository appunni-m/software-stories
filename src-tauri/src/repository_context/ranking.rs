//! Bounded candidate collection shared by lexical, identifier and declaration search.

use super::*;
use tantivy::{
    Order,
    collector::{
        Count, TopDocs,
        sort_key::{SortBySimilarityScore, SortByStaticFastValue, SortByString},
    },
};

type CandidateKey = (f32, (Option<String>, Option<u64>));

pub(super) fn candidates(
    searcher: &tantivy::Searcher,
    query: &dyn Query,
) -> tantivy::Result<(Vec<(f32, tantivy::DocAddress)>, usize)> {
    // Tie breaks must be applied inside TopDocs. Sorting the retained 256 hits
    // later cannot recover a lower path/offset discarded by document-address ties.
    let collector = TopDocs::with_limit(MAX_CANDIDATES).order_by((
        (SortBySimilarityScore, Order::Desc),
        (
            (SortByString::for_field("source_path"), Order::Asc),
            (
                SortByStaticFastValue::<u64>::for_field("start_byte"),
                Order::Asc,
            ),
        ),
    ));
    let (hits, count): (Vec<(CandidateKey, tantivy::DocAddress)>, usize) =
        searcher.search(query, &(collector, Count))?;
    let hits = hits
        .into_iter()
        .map(|((score, (path, start)), address)| {
            if path.is_none() || start.is_none() || !score.is_finite() {
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
