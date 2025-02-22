use std::collections::BTreeMap;

use p3_baby_bear::BabyBear;
use p3_field::AbstractField;
use p3_field::{Field, PrimeField64};
use p3_matrix::Matrix;

use crate::air::MachineAir;
use crate::runtime::ExecutionRecord;
use crate::stark::{ChipRef, StarkGenericConfig};

use super::InteractionKind;

#[derive(Debug)]
pub struct InteractionData<F: Field> {
    pub chip_name: String,
    pub kind: InteractionKind,
    pub row: usize,
    pub interaction_number: usize,
    pub is_send: bool,
    pub multiplicity: F,
}

pub fn vec_to_string<F: Field>(vec: Vec<F>) -> String {
    let mut result = String::from("(");
    for (i, value) in vec.iter().enumerate() {
        if i != 0 {
            result.push_str(", ");
        }
        result.push_str(&value.to_string());
    }
    result.push(')');
    result
}

fn babybear_to_int(n: BabyBear) -> i32 {
    let modulus = BabyBear::ORDER_U64;
    let val = n.as_canonical_u64();
    if val > modulus / 2 {
        val as i32 - modulus as i32
    } else {
        val as i32
    }
}

pub fn debug_interactions<SC: StarkGenericConfig>(
    chip: &ChipRef<SC>,
    record: &ExecutionRecord,
    interaction_kinds: Vec<InteractionKind>,
) -> (
    BTreeMap<String, Vec<InteractionData<SC::Val>>>,
    BTreeMap<String, SC::Val>,
) {
    let mut key_to_vec_data = BTreeMap::new();
    let mut key_to_count = BTreeMap::new();

    let trace = chip.generate_trace(record, &mut ExecutionRecord::default());
    let mut main = trace.clone();
    let height = trace.clone().height();

    let nb_send_interactions = chip.sends().len();
    for row in 0..height {
        for (m, interaction) in chip
            .sends()
            .iter()
            .chain(chip.receives().iter())
            .enumerate()
        {
            if !interaction_kinds.contains(&interaction.kind) {
                continue;
            }
            let is_send = m < nb_send_interactions;
            let multiplicity_eval: SC::Val = interaction.multiplicity.apply(&[], main.row_mut(row));

            if !multiplicity_eval.is_zero() {
                let mut values = vec![];
                for value in &interaction.values {
                    let expr: SC::Val = value.apply(&[], main.row_mut(row));
                    values.push(expr);
                }
                let key = format!(
                    "{} {}",
                    &interaction.kind.to_string(),
                    vec_to_string(values)
                );
                key_to_vec_data
                    .entry(key.clone())
                    .or_insert_with(Vec::new)
                    .push(InteractionData {
                        chip_name: chip.name(),
                        kind: interaction.kind,
                        row,
                        interaction_number: m,
                        is_send,
                        multiplicity: multiplicity_eval,
                    });
                let current = key_to_count.entry(key.clone()).or_insert(SC::Val::zero());
                if is_send {
                    *current += multiplicity_eval;
                } else {
                    *current -= multiplicity_eval;
                }
            }
        }
    }

    (key_to_vec_data, key_to_count)
}

/// Calculate the the number of times we send and receive each event of the given interaction type,
/// and print out the ones for which the set of sends and receives don't match.
pub fn debug_interactions_with_all_chips<SC: StarkGenericConfig<Val = BabyBear>>(
    chips: &[ChipRef<SC>],
    segment: &ExecutionRecord,
    interaction_kinds: Vec<InteractionKind>,
) -> bool {
    let mut final_map = BTreeMap::new();

    for chip in chips.iter() {
        let (_, count) = debug_interactions(chip, segment, interaction_kinds.clone());

        tracing::debug!("{} chip has {} distinct events", chip.name(), count.len());
        for (key, value) in count.iter() {
            let entry = final_map
                .entry(key.clone())
                .or_insert((SC::Val::zero(), BTreeMap::new()));
            entry.0 += *value;
            *entry.1.entry(chip.name()).or_insert(SC::Val::zero()) += *value;
        }
    }

    tracing::debug!("Final counts below.");
    tracing::debug!("==================");

    let mut any_nonzero = false;
    for (key, (value, chip_values)) in final_map.clone() {
        if !SC::Val::is_zero(&value) {
            tracing::debug!(
                "Interaction key: {} Send-Receive Discrepancy: {}",
                key,
                babybear_to_int(value)
            );
            any_nonzero = true;
            for (chip, chip_value) in chip_values {
                tracing::debug!(
                    " {} chip's send-receive discrepancy for this key is {}",
                    chip,
                    babybear_to_int(chip_value)
                );
            }
        }
    }

    tracing::debug!("==================");
    if !any_nonzero {
        tracing::debug!("All chips have the same number of sends and receives.");
    } else {
        tracing::debug!("Positive values mean sent more than received.");
        tracing::debug!("Negative values mean received more than sent.");
    }

    !any_nonzero
}
