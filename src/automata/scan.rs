use super::execution::Execution;
use super::id::StartId;

/// A match that [`longest_match`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<A> {
    /// The accept that the scan reached.
    pub accept: A,
    /// The number of the symbols in the match.
    pub length: usize,
}

/// Returns the longest match at the start of `input` under `start`.
///
/// The function returns `None` if the scan reaches no accept. If the scan reaches more than one
/// accept at the same length, the lowest accept is the result. Thus give the accepts in the
/// sequence of precedence.
///
/// Only the rules of `start` are applicable. The other start states take no part in the scan.
///
/// The function scans one token. Give the same execution for each token of the input. Thus the
/// scanner makes the buffers one time, and not one time for each token.
///
/// # Panics
///
/// This function panics if `start` is not a start state of the automaton.
pub fn longest_match<E: Execution>(
    execution: &mut E,
    start: StartId,
    input: &[E::Symbol],
) -> Option<Match<E::Accept>>
where
    E::Accept: Ord + Clone,
{
    execution.restart(start);
    let mut best = accepted(execution).map(|accept| Match { accept, length: 0 });

    for (consumed, &symbol) in input.iter().enumerate() {
        if !execution.step(symbol) {
            break;
        }
        best = accepted(execution)
            .map(|accept| Match {
                accept,
                length: consumed + 1,
            })
            .or(best);
    }

    best
}

/// Returns the lowest accept that `execution` reached.
fn accepted<E: Execution>(execution: &E) -> Option<E::Accept>
where
    E::Accept: Ord + Clone,
{
    execution.accepts().min().cloned()
}
