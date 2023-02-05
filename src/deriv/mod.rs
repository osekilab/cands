pub mod li;
pub mod lit;
pub mod so;



use crate::prelude::*;

use derive_more::{ Deref, DerefMut };

use std::collections::HashMap;
use std::marker::PhantomData;
use std::fmt;



/// Universal Grammar.
/// 
/// From Definition 1 in C&S 2016, p. 44.
/// 
/// >Universal Grammar is a 6-tuple: $\\langle \\textrm{PHON-F}, \\textrm{SYN-F}, \\textrm{SEM-F}, \\textrm{Select}, \\textrm{Merge}, \\textrm{Transfer} \\rangle$.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UniversalGrammar<T: Triggers> {
    pub phon_f:     Set<Feature>,
    pub syn_f:      Set<Feature>,
    pub sem_f:      Set<Feature>,
    t:              PhantomData<T>,
}

impl<T: Triggers> UniversalGrammar<T> {
    pub fn new(phon_f: Set<Feature>, syn_f: Set<Feature>, sem_f: Set<Feature>) -> Self {
        Self {
            phon_f, syn_f, sem_f, t: PhantomData::default()
        }
    }
}



/// Lexicon.
/// 
/// From Definition 3 in C&S 2016, p. 44.
/// 
/// >A *lexicon* is a finite set of lexical items.
pub type Lexicon = Set<LexicalItem>;



/// I-language.
/// 
/// From Definition 4 in C&S 2016, p. 45.
/// 
/// An I-language is a pair $\\langle \\textrm{Lex}, \\textrm{UG} \\rangle$ where $\\textrm{Lex}$ is a lexicon and $\\textrm{UG}$ is Universal Grammar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ILanguage<T: Triggers> {
    pub lex: Lexicon,
    pub ug: UniversalGrammar<T>,
}



/// Lexical array.
/// 
/// From Definition 6 in C&S 2016, p. 45.
/// 
/// >A *lexical array* (LA) is a finite set of lexical item tokens.
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LexicalArray(pub Set<LexicalItemToken>);



impl LexicalArray {
    pub fn new(set: Set<LexicalItemToken>) -> Self {
        LexicalArray(set)
    }
}



/// Workspace.
/// 
/// See [`Stage`].
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Workspace(pub Set<SyntacticObject>);



impl Workspace {
    pub fn new(set: Set<SyntacticObject>) -> Self {
        Workspace(set)
    }
}



macro_rules! w {
    ($($so:expr),*) => {
        Workspace::new(set!( $($so),* ))
    }
}

pub(crate) use w;



impl Workspace {
    /// Immediate containment.
    /// 
    /// Derived from Definition 8, C&S 2016, p. 46.
    /// 
    /// Something like:
    /// 
    /// >Let $W$ be a workspace and $A$ a syntactic object, then $W$ *immediately contains* $A$ iff $A \\in W$.
    pub fn immediately_contains(&self, so: &SyntacticObject) -> bool {
        self.0.contains(so)
    }

    /// Containment.
    /// 
    /// From Definition 9, C&S 2016, p. 46.
    /// 
    /// Something like:
    /// 
    /// >Let $W$ be a workspace and $A$ a syntactic object, then $W$ *contains* $A$ iff
    /// >
    /// >1.  $W$ immediately contains $A$, or
    /// >2.  for some syntactic object $B$, $W$ immediately contains $B$ and $B$ contains $A$.
    pub fn contains(&self, so: &SyntacticObject) -> bool {
        self.0.contains(so) ||
        self.0.iter()
            .any(|b| b.contains(so))
    }



    /// Return an iterator over all the syntactic objects contained in `self`.
    pub fn contained_sos(&self, pic_compliant: bool) -> ContainedSyntacticObjects {
        let stack = self.0.iter().collect();
        ContainedSyntacticObjects::new(stack, pic_compliant)
    }
}



/// An iterator over the syntactic objects contained in a `Workspace`.
pub struct ContainedSyntacticObjects<'a> {
    /// The stack of syntactic objects that this iterator is supposed to visit.
    stack: Vec<&'a SyntacticObject>,
    pic_compliant: bool,
}



impl<'a> ContainedSyntacticObjects<'a> {
    fn new(stack: Vec<&'a SyntacticObject>, pic_compliant: bool) -> Self {
        Self { stack, pic_compliant }
    }
}



impl<'a> Iterator for ContainedSyntacticObjects<'a> {
    type Item = &'a SyntacticObject;

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop()
            .map(|so| {
                match so {
                    &SyntacticObject::Set(ref children) => {
                        for child in children {
                            self.stack.push(child);
                        }
                    },
                    &SyntacticObject::Transfer { ref so, .. } => {
                        if !self.pic_compliant {
                            self.stack.push(so);
                        }
                    },
                    _ => (),
                }

                so
            })
    }
}



impl fmt::Display for Workspace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\n")?;
        for so in self.0.iter() {
            write!(f, "  {},\n", SOPrefixFormatter::new(so, 2))?;
        }
        write!(f, "}}")
    }
}



/// Stage.
/// 
/// From Definition 10, C&S 2016, p. 46.
/// 
/// >A *stage* is a pair $S = \\langle \textrm{LA}, W \\rangle$, where $\\textrm{LA}$ is a lexical array and $W$ is a set of syntactic objects. We call $W$ the *workspace* of $S$.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Stage {
    pub la: LexicalArray,
    pub w: Workspace,
}



/// >Derive-by-Select: for some $A \\in \\textrm{LA}\_i$, $\\langle \\textrm{LA}\_{i+1}, W\_{i+1} \\rangle = \\textrm{Select} ( A, \\langle \\textrm{LA}\_i, W\_i \\rangle )$.
#[logwrap::logwrap]
fn derive_by_select(stage1: &Stage, stage2: &Stage) -> bool {
    let Stage { la: la1, w: w1 } = stage1;
    let Stage { la: la2, w: w2 } = stage2;

    let res = la1.iter()
        .find(|&lit| {
            my_debug!("Check for the lexical item token:\n{}", lit);
            select(lit.clone(), stage1.clone())
                .map_or(false, |stage| {
                    //eprintln!("Test stage: {:?}", stage);
                    stage2 == &stage
                })
        });

    if let Some(lit) = res {
        my_info!("This pair of stages is derived by selecting:\n{}.", lit);
    }

    res.is_some()
}



/// >Derive-by-Merge: $\\textrm{LA}\_i = \\textrm{LA}\_{i+1}$ and the following conditions hold for some $A, B$:
/// >    1.  $A \\in W\_i$,
/// >    2.  either $A$ contains $B$ or $W\_i$ immediately contains $B$, and
/// >    3.  $W\_{i+1} = ( W\_i - \\{ A, B \\} ) \\cup \\{ \\textrm{Merge} ( A, B ) \\}$.
#[logwrap::logwrap]
fn derive_by_merge<T: Triggers>(stage1: &Stage, stage2: &Stage) -> bool {
    let Stage { la: la1, w: w1 } = stage1;
    let Stage { la: la2, w: w2 } = stage2;

    if la1 != la2 {
        my_debug!("The lexical arrays must be the same.");
        return false;
    }

    if w1.0.is_empty() {
        my_debug!("The first workspace in the pair cannot be empty.");
        return false;
    }

    /*
        Derive-by-Merge?

        Derive-by-Merge is satisfied if there is a pair A, B that satisfy 3 conditions, one of which is that A in Wi. So we just search for a pair A, B where A is immediately contained in Wi and A, B satisfy the 2 other conditions. This works only if Wi is not empty, so we check that separately.
    */
    my_debug!("Search for a possible pair A, B to form Merge(A, B)...");
    my_debug!("Search for A. Iterate over all roots in the first workspace in the pair...");
    w1.0.iter()
        //  Iterate over all A, i.e., all root SOs in W1.
        .any(|a| {
            my_debug!(
                "Try A = {}",
                SOPrefixFormatter::new(a, 8)
            );
            inc!();
            //  Iterate over some B, i.e. all SOs contained in A ...
            my_debug!("Search for B. Iterate over all SOs contained by A...");
            let res = a.contained_sos(false, true)
                //  ... as well as all SOs immediately contained in W1.
                .chain(w1.0.iter())
                //  Check if the final condition match.
                .find(|&b| {
                    my_debug!(
                        "Try B = {}",
                        SOPrefixFormatter::new(b, 8)
                    );
                    let mut w = w1.clone();
                    w.0.remove(a);
                    w.0.remove(b);

                    triggered_merge::<T>(a.clone(), b.clone(), w1)
                        // .map_or_else(
                        //     |e| {
                        //         my_debug!("Merge failed with the following error: {}", e);
                        //         false
                        //     },
                        //     move |ab| {
                        .map_or(false, move |ab| {
                                my_debug!("Merge(A, B) works.");
                                my_debug!(
                                    "Merge(A, B) = {}",
                                    SOPrefixFormatter::new(&ab, 14)
                                );
                                w.0.insert(ab);
                                w2 == &w
                            }
                        )
                });
            dec!();

            if let Some(b) = res {
                my_info!("This pair of stages is derived by Merge(A, B),");
                my_info!("where A = {}", SOPrefixFormatter::new(&a, 10));
                my_info!("  and B = {}", SOPrefixFormatter::new(b, 10));
            }

            res.is_some()
        })
}



/// >Derive-by-Merge: $\\textrm{LA}\_i = \\textrm{LA}\_{i+1}$ and the following conditions hold for some $A, B$:
/// >    1.  $A \\in W\_i$,
/// >    2.  either $A$ contains $B$ or $W\_i$ immediately contains $B$, and
/// >    3.  $W\_{i+1} = ( W\_i - \\{ A, B \\} ) \\cup \\{ \\textrm{Merge} ( A, B ) \\}$.
#[logwrap::logwrap]
fn derive_by_transfer<T: Triggers>(stage1: &Stage, stage2: &Stage) -> bool {
    let Stage { la: la1, w: w1 } = stage1;
    let Stage { la: la2, w: w2 } = stage2;

    if la1 != la2 {
        my_debug!("The lexical arrays must be the same.");
        return false;
    }

    if w1.0.is_empty() {
        my_debug!("The first workspace in the pair cannot be empty.");
        return false;
    }

    my_debug!("Search for a strong phase...");
    w1.0.iter()
        .any(|so1| {
            //  Is a strong phase...
            is_strong_phase::<T>(so1, w1) &&
            //  Containing no other...
            {
                my_debug!(
                    "SO1 is a strong phase: {}",
                    SOPrefixFormatter::new(so1, 23)
                );

                // !so1.contained_sos(false, true)
                //     .any(|so2| {
                //         //  Strong phases...
                //         is_strong_phase::<T>(so2, w1) &&
                //         //  whose...
                //         match T::label_of(so2, w1) {
                //             Ok(lit) => {
                //                 let head = so!(lit.clone());

                //                 so2.contained_sos(true, true)
                //                     .any(|so3| {
                //                         match so3 {
                //                             &SyntacticObject::Set(ref vec) => {
                //                                 vec.iter()
                //                                     .any(|so4| {
                //                                         //  complement...
                //                                         so4.is_complement_of::<T>(&head, so3, w1) &&
                //                                         //  has not yet been transferred...
                //                                         match so4 {
                //                                             &SyntacticObject::Transfer { .. } => false,
                //                                             _ => true,
                //                                         }
                //                                     })
                //                             },
                //                             _ => false,
                //                         }
                //                     })
                //             },
                //             _ => false,
                //         }
                //     })

                let exception: Option<(&SyntacticObject, &SyntacticObject)> =
                    so1.contained_sos(false, true)
                        //  Look for a strong phase SO2.
                        .filter(|&so2| {
                            is_strong_phase::<T>(so2, w1)
                        })
                        //  Find the head of each SO2.
                        .filter_map(|so2| {
                            match T::label_of(so2, w1) {
                                Ok(label) => {
                                    Some((so2, label))
                                },
                                _ => None,
                            }
                        })
                        //  Iterate over each child SO3 of SO2.
                        .map(|(so2, label)| {
                            let so3s = so2.contained_sos(true, true);
                            (so2, label, so3s)
                        })
                        //  If SO3 = { ... }, iterate over each child SO4 of
                        //  SO3 such that SO4 is the complement of SO3.
                        .map(|(so2, label, so3s)| {
                            let so4s = so3s
                                .filter_map(move |so3| {
                                    match so3 {
                                        &SyntacticObject::Set(ref vec) => {
                                            Some(vec.iter()
                                                .filter(move |&so4| {
                                                    let head = so!(label.clone());
                                                    so4.is_complement_of::<T>(&head, so3, w1)
                                                }))
                                        },
                                        _ => None,
                                    }
                                })
                                .flatten();
                            (so2, so4s)
                        })
                        //  We have an exception if SO4 is not yet transferred.
                        .map(|(so2, so4s)| {
                            (so2, so4s.filter(|so4| {
                                match so4 {
                                    &SyntacticObject::Transfer { .. } => false,
                                    _ => true,
                                }
                            }))
                        })
                        //  Flatten!
                        .map(|(so2, mut so4s)| {
                            (so2, so4s.next())
                        })
                        .filter(|(so2, so4)| {
                            so4.is_some()
                        })
                        .map(|(so2, so4)| {
                            (so2, so4.unwrap())
                        })
                        .next();

                if let Some((so2, so4)) = exception {
                    my_debug!(
                        "SO1 contains a strong phase SO2 = {}",
                        SOPrefixFormatter::new(so2, 34)
                    );
                    my_debug!(
                        "whose complement SO4, shown below, is not transferred:\n{}",
                        so4
                    );
                }
                else {
                    my_debug!("SO1 does not contain a strong phase whose complement has not yet been transferred.");
                }

                true
            } &&
            //  And either...
            ({
                // eprintln!("Derivation: Try Transfer ::::::::::::::::::::::::::::::::");
                my_debug!("Try Transfer(SO1, SO1)...");
                let mut w = w1.clone();
                w.0.remove(so1);
                w.0.insert(transfer::<T>(&so1, so1.clone(), w1));
                my_debug!("The workspace should be: {}", w);
                
                let res = w == *w2;

                if res {
                    my_info!("This pair of stages is derived by Transfer(SO1, SO1).");
                }

                res
            } || {
                // eprintln!("Derivation: Try Cyclic-Transfer :::::::::::::::::::::::::");
                my_debug!("Try Cyclic-Transfer(SO1)...");
                let mut w = w1.clone();
                w.0.remove(so1);
                match cyclic_transfer::<T>(so1.clone(), w1) {
                    Ok(so2) => {
                        w.0.insert(so2);
                        my_debug!("The workspace should be: {}", w);
                        
                        let res = w == *w2;

                        if res {
                            my_info!("This pair of stages is derived by Cyclic-Transfer(SO1).");
                        }

                        res
                    },
                    Err(_) => false,
                }
            })
        })
}



/*
    For X, Y both equal to or contained in some [Spec; TP] in SO,
    the number of occurrences of X must equal the number of occurrences of Y.

    The way `contained_sos` visits SOs guarantee that it counts the number of
    occurrences of each SO.
 */
fn contains_bad_chain<'a, T: Triggers>(so: &'a SyntacticObject, w: &Workspace) -> Result<(), HashMap<&'a SyntacticObject, usize>> {
    let spec_tps: Set<_> = so.contained_sos(true, true)
        //  Transform into all [Spec; TP]s.
        .filter_map(|so| {
            match so {
                SyntacticObject::Set(set) => {
                    match set.len() == 2 {
                        true => {
                            let mut it = set.iter();

                            let x1 = it.next().unwrap();
                            let x2 = it.next().unwrap();
                            assert!(it.next().is_none());

                            //  Maybe x1 is Spec, and x2 is T(P)?
                            if x1.is_specifier_of::<T>(x2, so, w) {
                                if let Ok(label) = T::label_of(x2, w) {
                                    if label.li.syn.contains(&tense_feature!()) {
                                        return Some(x1);
                                    }
                                }
                            }

                            //  Maybe x2 is Spec, and x1 is T(P)?
                            if x2.is_specifier_of::<T>(x1, so, w) {
                                if let Ok(label) = T::label_of(x1, w) {
                                    if label.li.syn.contains(&tense_feature!()) {
                                        return Some(x2);
                                    }
                                }
                            }

                            None
                        },
                        false => None,
                    }
                },
                _ => None,
            }
        })
        //  Get all SOs contained in some [Spec; TP]s.
        .collect();

    //  Occurrences of all SOs contained in some [Spec; TP] (not necessarily
    //  the same!) in `so`.
    let cnts: HashMap<&'a SyntacticObject, usize> =
        so.contained_sos(true, true)
            //  We can take out this filter and it will still work, but it will
            //  reduce the search space later.
            .filter(|so| {
                spec_tps.iter()
                    .any(|&spec_tp| spec_tp.contains(so))
            })
            .fold(HashMap::new(), |mut map, so| {
                match map.get_mut(&so) {
                    Some(cnt) => { *cnt += 1; },
                    None => { map.insert(so, 1); },
                }
                map
            });

    for spec_tp in spec_tps {
        let mut same_cnt = None;
        for (&so, cnt) in cnts.iter() {
            if spec_tp.contains(so) {
                match same_cnt {
                    Some(same_cnt) => {
                        if same_cnt != *cnt {
                            return Err(cnts);
                        }
                    },
                    None => {
                        same_cnt = Some(*cnt);
                    },
                }
            }
        }
    }
    
    Ok(())
}



/// Check if the sequence of stages `stages` is a derivation from the I-language `il`.
/// 
/// From Definition 14, C&S 2016, p. 48. The original derivation, given below, defines a derivation with respect to just a lexicon, but since it invokes syntactic operations like Select and Merge, we define it with respect to an I-language, which includes a UG as well as a lexicon.
/// 
/// >A *derivation* from lexicon $L$ is a finite sequence of stages $\\langle S\_1, \ldots, S\_n \\rangle$ for $n \\geq 1$, where each $S\_i = \\langle \\textrm{LA}\_i, W\_i \\rangle$, such that
/// >
/// >1.  For all $\\textrm{LI}$ and $k$ such that $\\langle \\textrm{LI}$, $k \\rangle \\in \\textrm{LA}_1$, $\\textrm{LI} \\in L$,
/// >2.  $W\_1 = \\{ \\}$ (the empty set)
/// >3.  for all $i$, such that $1 \\leq i < n$, either:
/// >    *   Derive-by-Select, or
/// >    *   Derive-by-Merge, or
/// >    *   Derive-by-Transfer.
#[logwrap::logwrap]
pub fn is_derivation<T: Triggers>(il: &ILanguage<T>, stages: &[Stage]) -> bool {
    //  A derivation must have positive length.
    my_debug!("Step 1: Check if the derivation has positive length, i.e. that it has a positive number of stages...");
    if stages.len() < 1 {
        my_info!("The derivation must have >= 1 stages.");
        return false;
    }


    
    //  Check if all lexical item tokens at the first stage are in the lexicon.
    my_debug!("Step 2: Check if all the lexical item tokens in the lexical array of the first stage are in the lexicon...");
    
    let stage1 = &stages[0];
    let Stage { la: la1, w: w1 } = stage1;

    let ILanguage { lex, ug, .. } = il;

    for lit in la1.0.iter() {
        let LexicalItemToken { li, .. } = lit;
        if !lex.contains(li) {
            my_info!("Can't find this lexical item in the lexicon: {}", li);
            return false;
        }
    }

    //  Check if the workspace at the first stage is empty.
    my_debug!("Step 3: Check if the workspace of the first stage is empty...");

    if !w1.0.is_empty() {
        eprintln!("The first workspace must be empty.");
        // return false;
        return false;
    }

    //  Check every stage.
    my_debug!("Step 4: Check if every (non-first) stage is derivable from the previous stage...");

    for (stage_idx, stage_pair) in stages.windows(2).enumerate() {
        let fst_stage_idx = stage_idx + 1;
        let snd_stage_idx = stage_idx + 2;

        //  Report the current pair of stages
        my_debug!("============================================================");
        my_debug!("Checking the pair ({}, {})...", fst_stage_idx, snd_stage_idx);

        let stage1 = &stage_pair[0];
        let stage2 = &stage_pair[1];

        my_debug!("------------------------------------------------------------");
        my_debug!(
            "Lexical array {}: {{{}\n}}",
            fst_stage_idx,
            stage1.la.iter()
                .map(|lit| format!("  {},", lit))
                .fold(format!(""), |a, b| format!("{}\n{}", a, b))
        );
        my_debug!(
            "Workspace {}: {}",
            fst_stage_idx,
            stage1.w
        );

        my_debug!("------------------------------------------------------------");
        my_debug!(
            "Lexical array {}: {{{}\n}}",
            snd_stage_idx,
            stage2.la.iter()
                .map(|lit| format!("  {},", lit))
                .fold(format!(""), |a, b| format!("{}\n{}", a, b))
        );
        my_debug!(
            "Workspace {}: {}",
            snd_stage_idx,
            stage2.w
        );

        let step_ok = loop {
            //  Derive-by-Select?
            my_debug!("------------------------------------------------------------");
            my_debug!("Check for Derive-by-Select...");
            if derive_by_select(stage1, stage2) {
                my_debug!("Match!");
                break true;
            }
            my_debug!("No match.");

            //  Derive-by-Merge?

            my_debug!("------------------------------------------------------------");
            my_debug!("Check for Derive-by-Merge...");
            if derive_by_merge::<T>(stage1, stage2) {
                my_debug!("Match!");
                break true;
            }
            my_debug!("No match.");

            //  Derive-by-Transfer?
            my_debug!("------------------------------------------------------------");
            my_debug!("Check for Derive-by-Transfer...");
            if derive_by_transfer::<T>(stage1, stage2) {
                my_debug!("Match!");
                break true;
            }
            my_debug!("No match.");

            break false;
        };

        if !step_ok {
            eprintln!("This pair of stages is invalid:");
            eprintln!("Stage 1:\n");
            eprintln!(
                " :: Lexical array: {{{}\n}}",
                stage1.la.iter()
                    .map(|lit| format!("{}", lit))
                    .fold(format!(""), |a, b| format!("{}\n{}", a, b))
            );
            eprintln!(" :: Workspace:\n{}", stage1.w);
            eprintln!("Stage 2:\n");
            eprintln!(
                " :: Lexical array: {{{}\n}}",
                stage2.la.iter()
                    .map(|lit| format!("{}", lit))
                    .fold(format!(""), |a, b| format!("{}\n{}", a, b))
            );
            eprintln!(" :: Workspace:\n{}", stage2.w);
            return false;
        }
        
        my_debug!("------------------------------------------------------------");
        my_debug!("Checking if the 2nd workspace contains a bad chain...");

        let bad_chain = stage2.w.iter()
            .map(|root| contains_bad_chain::<T>(root, &stage2.w))
            .fold(Ok(()), |res1, res2| {
                res1.and(res2)
            });

        if let Err(cnts) = bad_chain {
            eprintln!("The 2nd workspace contains a bad chain:");
            eprintln!(" :: Workspace: {}", stage2.w);
            eprintln!("Bad chains:");
            for (so, cnt) in cnts {
                eprintln!(" :: Appeared {} times:", cnt);
                eprintln!(" :: {}", SOPrefixFormatter::new(so, 4));
            }
            return false;
        }
    }

    true
}