//! Merging the combinators of selectors being woven together.

use super::*;

/// Extracts leading `Combinator`s from `components_one` and `components_two` and
/// merges them together into a single list of combinators.
///
/// If there are no combinators to be merged, returns an empty list. If the
/// combinators can't be merged, returns `None`.
pub(super) fn merge_initial_combinators(
    components_one: &mut VecDeque<ComplexSelectorComponent>,
    components_two: &mut VecDeque<ComplexSelectorComponent>,
) -> Option<Vec<Combinator>> {
    let mut combinators_one: Vec<Combinator> = Vec::new();

    while let Some(ComplexSelectorComponent::Combinator(c)) = components_one.front() {
        combinators_one.push(*c);
        components_one.pop_front();
    }

    let mut combinators_two = Vec::new();

    while let Some(ComplexSelectorComponent::Combinator(c)) = components_two.front() {
        combinators_two.push(*c);
        components_two.pop_front();
    }

    let lcs = longest_common_subsequence(&combinators_one, &combinators_two, None);

    if lcs == combinators_one {
        Some(combinators_two)
    } else if lcs == combinators_two {
        Some(combinators_one)
    } else {
        // If neither sequence of combinators is a subsequence of the other, they
        // cannot be merged successfully.
        None
    }
}

/// Returns the longest common subsequence between `list_one` and `list_two`.
///
/// If there are more than one equally long common subsequence, returns the one
/// which starts first in `list_one`.
///
/// If `select` is passed, it's used to check equality between elements in each
/// list. If it returns `None`, the elements are considered unequal; otherwise,
/// it should return the element to include in the return value.
pub(super) fn longest_common_subsequence<T: PartialEq + Clone>(
    list_one: &[T],
    list_two: &[T],
    select: Option<&dyn Fn(T, T) -> Option<T>>,
) -> Vec<T> {
    let select = select.unwrap_or(&|element_one, element_two| {
        if element_one == element_two {
            Some(element_one)
        } else {
            None
        }
    });

    let mut lengths = vec![vec![0; list_two.len() + 1]; list_one.len() + 1];

    let mut selections: Vec<Vec<Option<T>>> = vec![vec![None; list_two.len()]; list_one.len()];

    for i in 0..list_one.len() {
        for j in 0..list_two.len() {
            let selection = select(
                list_one.get(i).unwrap().clone(),
                list_two.get(j).unwrap().clone(),
            );
            selections[i][j] = selection.clone();
            lengths[i + 1][j + 1] = if selection.is_none() {
                std::cmp::max(lengths[i + 1][j], lengths[i][j + 1])
            } else {
                lengths[i][j] + 1
            };
        }
    }

    pub(super) fn backtrack<T: Clone>(
        i: isize,
        j: isize,
        lengths: Vec<Vec<i32>>,
        selections: &mut Vec<Vec<Option<T>>>,
    ) -> Vec<T> {
        if i == -1 || j == -1 {
            return Vec::new();
        }

        let selection = selections.get(i as usize).cloned().unwrap_or_default();

        if let Some(Some(selection)) = selection.get(j as usize) {
            let mut tmp = backtrack(i - 1, j - 1, lengths, selections);
            tmp.push(selection.clone());
            return tmp;
        }

        if lengths[(i + 1) as usize][j as usize] > lengths[i as usize][(j + 1) as usize] {
            backtrack(i, j - 1, lengths, selections)
        } else {
            backtrack(i - 1, j, lengths, selections)
        }
    }
    backtrack(
        (list_one.len() as isize).saturating_sub(1),
        (list_two.len() as isize).saturating_sub(1),
        lengths,
        &mut selections,
    )
}

/// Extracts trailing `Combinator`s, and the selectors to which they apply, from
/// `components_one` and `components_two` and merges them together into a single list.
///
/// If there are no combinators to be merged, returns an empty list. If the
/// sequences can't be merged, returns `None`.
#[allow(clippy::cognitive_complexity)]
pub(super) fn merge_final_combinators(
    components_one: &mut VecDeque<ComplexSelectorComponent>,
    components_two: &mut VecDeque<ComplexSelectorComponent>,
    result: Option<VecDeque<Vec<Vec<ComplexSelectorComponent>>>>,
) -> Option<Vec<Vec<Vec<ComplexSelectorComponent>>>> {
    let mut result = result.unwrap_or_default();

    if (components_one.is_empty() || !components_one.back().unwrap().is_combinator())
        && (components_two.is_empty() || !components_two.back().unwrap().is_combinator())
    {
        return Some(Vec::from(result));
    }

    let mut combinators_one = Vec::new();

    while let Some(ComplexSelectorComponent::Combinator(combinator)) =
        components_one.get(components_one.len().saturating_sub(1))
    {
        combinators_one.push(*combinator);
        components_one.pop_back();
    }

    let mut combinators_two = Vec::new();

    while let Some(ComplexSelectorComponent::Combinator(combinator)) =
        components_two.get(components_two.len().saturating_sub(1))
    {
        combinators_two.push(*combinator);
        components_two.pop_back();
    }

    if combinators_one.len() > 1 || combinators_two.len() > 1 {
        // If there are multiple combinators, something hacky's going on. If one
        // is a supersequence of the other, use that, otherwise give up.
        let lcs = longest_common_subsequence(&combinators_one, &combinators_two, None);
        if lcs == combinators_one {
            result.push_front(vec![
                combinators_two
                    .into_iter()
                    .map(ComplexSelectorComponent::Combinator)
                    .rev()
                    .collect(),
            ]);
        } else if lcs == combinators_two {
            result.push_front(vec![
                combinators_one
                    .into_iter()
                    .map(ComplexSelectorComponent::Combinator)
                    .rev()
                    .collect(),
            ]);
        } else {
            return None;
        }

        return Some(Vec::from(result));
    }

    let combinator_one = combinators_one.first();

    let combinator_two = combinators_two.first();

    // This code looks complicated, but it's actually just a bunch of special
    // cases for interactions between different combinators.
    match (combinator_one, combinator_two) {
        (Some(combinator_one), Some(combinator_two)) => {
            let compound_one = match components_one.pop_back() {
                Some(ComplexSelectorComponent::Compound(c)) => c,
                Some(..) | None => unreachable!(),
            };
            let compound_two = match components_two.pop_back() {
                Some(ComplexSelectorComponent::Compound(c)) => c,
                Some(..) | None => unreachable!(),
            };

            match (combinator_one, combinator_two) {
                (Combinator::FollowingSibling, Combinator::FollowingSibling) => {
                    if compound_one.is_super_selector(&compound_two, &None) {
                        result.push_front(vec![vec![
                            ComplexSelectorComponent::Compound(compound_two),
                            ComplexSelectorComponent::Combinator(Combinator::FollowingSibling),
                        ]]);
                    } else if compound_two.is_super_selector(&compound_one, &None) {
                        result.push_front(vec![vec![
                            ComplexSelectorComponent::Compound(compound_one),
                            ComplexSelectorComponent::Combinator(Combinator::FollowingSibling),
                        ]]);
                    } else {
                        let mut choices = vec![
                            vec![
                                ComplexSelectorComponent::Compound(compound_one.clone()),
                                ComplexSelectorComponent::Combinator(Combinator::FollowingSibling),
                                ComplexSelectorComponent::Compound(compound_two.clone()),
                                ComplexSelectorComponent::Combinator(Combinator::FollowingSibling),
                            ],
                            vec![
                                ComplexSelectorComponent::Compound(compound_two.clone()),
                                ComplexSelectorComponent::Combinator(Combinator::FollowingSibling),
                                ComplexSelectorComponent::Compound(compound_one.clone()),
                                ComplexSelectorComponent::Combinator(Combinator::FollowingSibling),
                            ],
                        ];

                        if let Some(unified) = compound_one.unify(compound_two) {
                            choices.push(vec![
                                ComplexSelectorComponent::Compound(unified),
                                ComplexSelectorComponent::Combinator(Combinator::FollowingSibling),
                            ]);
                        }

                        result.push_front(choices);
                    }
                }
                (Combinator::FollowingSibling, Combinator::NextSibling)
                | (Combinator::NextSibling, Combinator::FollowingSibling) => {
                    let following_sibling_selector =
                        if combinator_one == &Combinator::FollowingSibling {
                            compound_one.clone()
                        } else {
                            compound_two.clone()
                        };

                    let next_sibling_selector = if combinator_one == &Combinator::FollowingSibling {
                        compound_two.clone()
                    } else {
                        compound_one.clone()
                    };

                    if following_sibling_selector.is_super_selector(&next_sibling_selector, &None) {
                        result.push_front(vec![vec![
                            ComplexSelectorComponent::Compound(next_sibling_selector),
                            ComplexSelectorComponent::Combinator(Combinator::NextSibling),
                        ]]);
                    } else {
                        let mut v = vec![vec![
                            ComplexSelectorComponent::Compound(following_sibling_selector),
                            ComplexSelectorComponent::Combinator(Combinator::FollowingSibling),
                            ComplexSelectorComponent::Compound(next_sibling_selector),
                            ComplexSelectorComponent::Combinator(Combinator::NextSibling),
                        ]];

                        if let Some(unified) = compound_one.unify(compound_two) {
                            v.push(vec![
                                ComplexSelectorComponent::Compound(unified),
                                ComplexSelectorComponent::Combinator(Combinator::NextSibling),
                            ]);
                        }
                        result.push_front(v);
                    }
                }
                (Combinator::Child, Combinator::NextSibling)
                | (Combinator::Child, Combinator::FollowingSibling) => {
                    result.push_front(vec![vec![
                        ComplexSelectorComponent::Compound(compound_two),
                        ComplexSelectorComponent::Combinator(*combinator_two),
                    ]]);
                    components_one.push_back(ComplexSelectorComponent::Compound(compound_one));
                    components_one
                        .push_back(ComplexSelectorComponent::Combinator(Combinator::Child));
                }
                (Combinator::NextSibling, Combinator::Child)
                | (Combinator::FollowingSibling, Combinator::Child) => {
                    result.push_front(vec![vec![
                        ComplexSelectorComponent::Compound(compound_one),
                        ComplexSelectorComponent::Combinator(*combinator_one),
                    ]]);
                    components_two.push_back(ComplexSelectorComponent::Compound(compound_two));
                    components_two
                        .push_back(ComplexSelectorComponent::Combinator(Combinator::Child));
                }
                (..) => {
                    if combinator_one != combinator_two {
                        return None;
                    }

                    let unified = compound_one.unify(compound_two)?;

                    result.push_front(vec![vec![
                        ComplexSelectorComponent::Compound(unified),
                        ComplexSelectorComponent::Combinator(*combinator_one),
                    ]]);
                }
            }

            merge_final_combinators(components_one, components_two, Some(result))
        }
        (Some(combinator_one), None) => {
            if *combinator_one == Combinator::Child
                && !components_two.is_empty()
                && let Some(ComplexSelectorComponent::Compound(c1)) = components_one.back()
                && let Some(ComplexSelectorComponent::Compound(c2)) = components_two.back()
                && c2.is_super_selector(c1, &None)
            {
                components_two.pop_back();
            }

            result.push_front(vec![vec![
                components_one.pop_back().unwrap(),
                ComplexSelectorComponent::Combinator(*combinator_one),
            ]]);

            merge_final_combinators(components_one, components_two, Some(result))
        }
        (None, Some(combinator_two)) => {
            if *combinator_two == Combinator::Child
                && !components_one.is_empty()
                && let Some(ComplexSelectorComponent::Compound(c1)) = components_one.back()
                && let Some(ComplexSelectorComponent::Compound(c2)) = components_two.back()
                && c1.is_super_selector(c2, &None)
            {
                components_one.pop_back();
            }

            result.push_front(vec![vec![
                components_two.pop_back().unwrap(),
                ComplexSelectorComponent::Combinator(*combinator_two),
            ]]);
            merge_final_combinators(components_one, components_two, Some(result))
        }
        (None, None) => unreachable!(),
    }
}
