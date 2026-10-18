//! Match model for AZO decompression.
use crate::model::{BoolState, EntropyBitProb, PredictProb};
use crate::range::RangeDecoder;
use crate::recent::RecentList;
use crate::table::{CODE_SIZE, CodeTable};

const RECENT_DIST_SLOTS: usize = 2;
const MATCH_SLOTS: usize = 128;
const MIN_DISTANCE: u32 = 1;
const MIN_LENGTH: u32 = 2;

pub struct MatchCode {
    reuse_flag: BoolState,
    dist_recent_flag: BoolState,
    match_recent_flag: BoolState,

    dist_recent_sel: EntropyBitProb,
    dist_code_model: EntropyBitProb,
    match_recent_sel: EntropyBitProb,
    match_id_model: EntropyBitProb,

    length_model: PredictProb,

    recent_dists: RecentList<u32>,
    recent_matches: RecentList<u32>,
    match_lengths: RecentList<u32>,
    match_starts: RecentList<u32>,

    dist_table: CodeTable,
    length_table: CodeTable,
}

impl Default for MatchCode {
    fn default() -> Self {
        Self::new()
    }
}

impl MatchCode {
    pub fn new() -> Self {
        let recent_dists: Vec<u32> = (0..RECENT_DIST_SLOTS as u32)
            .map(|i| MIN_DISTANCE + i)
            .collect();
        let recent_matches: Vec<u32> = (0..RECENT_DIST_SLOTS as u32).collect();
        let match_lengths: Vec<u32> = (0..MATCH_SLOTS as u32).map(|i| MIN_LENGTH + i).collect();
        let match_starts: Vec<u32> = vec![0; MATCH_SLOTS];

        MatchCode {
            reuse_flag: BoolState::new(8),
            dist_recent_flag: BoolState::new(8),
            match_recent_flag: BoolState::new(8),

            dist_recent_sel: EntropyBitProb::new(RECENT_DIST_SLOTS),
            dist_code_model: EntropyBitProb::new(CODE_SIZE),
            match_recent_sel: EntropyBitProb::new(RECENT_DIST_SLOTS),
            match_id_model: EntropyBitProb::new(MATCH_SLOTS),

            length_model: PredictProb::new(CODE_SIZE, CODE_SIZE, 4),

            recent_dists: RecentList::new(recent_dists),
            recent_matches: RecentList::new(recent_matches),
            match_lengths: RecentList::new(match_lengths),
            match_starts: RecentList::new(match_starts),

            dist_table: CodeTable::build_dist_table(),
            length_table: CodeTable::build_length_table(),
        }
    }

    /// Decode a match at output position `write_pos`; returns `(distance, length)`.
    pub fn decode(&mut self, entropy: &mut RangeDecoder, write_pos: u32) -> (u32, u32) {
        if self.reuse_flag.decode(entropy) == 1 {
            self.reuse_recent_match(entropy, write_pos)
        } else {
            let distance = self.decode_distance(entropy);
            let length = self.decode_length(entropy, distance);
            self.match_lengths.push(length);
            self.match_starts.push(write_pos);
            (distance, length)
        }
    }

    /// Reuse a remembered `(length, start)` pair, deriving the distance from the
    /// current output position.
    fn reuse_recent_match(&mut self, entropy: &mut RangeDecoder, write_pos: u32) -> (u32, u32) {
        let match_id = if self.match_recent_flag.decode(entropy) != 0 {
            let slot = self.match_recent_sel.decode(entropy) as usize;
            self.recent_matches.promote(slot)
        } else {
            let id = self.match_id_model.decode(entropy);
            self.recent_matches.push(id);
            id
        } as usize;

        let length = self.match_lengths.promote(match_id);
        let start = self.match_starts.promote(match_id);
        let distance = write_pos.wrapping_sub(start);
        (distance, length)
    }

    fn decode_distance(&mut self, entropy: &mut RangeDecoder) -> u32 {
        if self.dist_recent_flag.decode(entropy) != 0 {
            let slot = self.dist_recent_sel.decode(entropy) as usize;
            self.recent_dists.promote(slot)
        } else {
            let code = self.dist_code_model.decode(entropy) as usize;
            let mut distance = self.dist_table.base[code];
            if self.dist_table.extra_bits[code] > 0 {
                distance += entropy.decode_uniform(self.dist_table.extra_bits[code]);
            }
            self.recent_dists.push(distance);
            distance
        }
    }

    fn decode_length(&mut self, entropy: &mut RangeDecoder, distance: u32) -> u32 {
        let dist_code = self.dist_table.code_for(distance);
        let code = self.length_model.decode(entropy, dist_code) as usize;
        let mut length = self.length_table.base[code];
        if self.length_table.extra_bits[code] > 0 {
            length += entropy.decode_uniform(self.length_table.extra_bits[code]);
        }
        length
    }
}
