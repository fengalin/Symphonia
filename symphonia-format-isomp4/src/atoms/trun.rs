// Symphonia
// Copyright (c) 2020 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia_core::errors::{Result, decode_error};
use symphonia_core::io::ByteStream;
use symphonia_core::util::bits;

use crate::atoms::{Atom, AtomHeader};

use log::warn;

/// Track fragment run atom.
#[derive(Debug)]
pub struct TrunAtom {
    /// Atom header.
    header: AtomHeader,
    /// Extended header flags.
    flags: u32,
    /// Data offset of this run.
    pub data_offset: Option<i32>,
    /// Number of samples in this run.
    pub sample_count: u32,
    /// Sample flags for the first sample only.
    pub first_sample_flags: Option<u32>,
    /// Sample duration for each sample in this run.
    pub sample_duration: Vec<u32>,
    /// Sample size for each sample in this run.
    pub sample_size: Vec<u32>,
    /// Sample flags for each sample in this run.
    pub sample_flags: Vec<u32>,
    /// The total size of all samples in this run. 0 if the sample size flag is not set.
    pub total_sample_size: u64,
    /// The total duration of all samples in this run. 0 if the sample duration flag is not set.
    pub total_sample_duration: u64,
}

impl TrunAtom {
    const SAMPLE_DURATION_PRESENT:u32 = 0x100;
    const SAMPLE_SIZE_PRESENT: u32 = 0x200;
    const SAMPLE_FLAGS_PRESENT: u32 = 0x400;
    const SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT: u32 = 0x800;

    /// Indicates if sample durations are provided.
    pub fn is_sample_duration_present(&self) -> bool {
        self.flags & TrunAtom::SAMPLE_DURATION_PRESENT != 0
    }

    /// Indicates if sample sizes are provided.
    pub fn is_sample_size_present(&self) -> bool {
        self.flags & TrunAtom::SAMPLE_SIZE_PRESENT != 0
    }

    /// Indicates if sample flags are provided.
    pub fn are_sample_flags_present(&self) -> bool {
        self.flags & TrunAtom::SAMPLE_FLAGS_PRESENT != 0
    }

    /// Indicates if sample composition time offsets are provided.
    pub fn are_sample_composition_time_offsets_present(&self) -> bool {
        self.flags & TrunAtom::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT != 0
    }
}

impl Atom for TrunAtom {
    fn header(&self) -> AtomHeader {
        self.header
    }

    fn read<B: ByteStream>(reader: &mut B, header: AtomHeader) -> Result<Self> {
        // Track fragment run atom flags.
        const DATA_OFFSET_PRESENT: u32 = 0x1;
        const FIRST_SAMPLE_FLAGS_PRESENT: u32 = 0x4;

        let (_, flags) = AtomHeader::read_extra(reader)?;

        let sample_count = reader.read_be_u32()?;

        let data_offset = match flags & DATA_OFFSET_PRESENT {
            0 => None,
            _ => Some(bits::sign_extend_leq32_to_i32(reader.read_be_u32()?, 32)),
        };

        let first_sample_flags = match flags & FIRST_SAMPLE_FLAGS_PRESENT {
            0 => None,
            _ => Some(reader.read_be_u32()?),
        };

        // Remember to implement support for truns with first-sample-flags-present.
        if first_sample_flags.is_some() {
            todo!("support truns with first-sample-flags-present");
        }

        // If the first-sample-flags-present flag is set, then the sample-flags-present flag should
        // not be set. The samples after the first shall use the default sample flags defined in the
        // tfhd or mvex atoms.
        if first_sample_flags.is_some() && (flags & TrunAtom::SAMPLE_FLAGS_PRESENT != 0) {
            return decode_error("sample-flag-present and first-sample-flags-present flags are set");
        }

        let mut sample_duration = Vec::new();
        let mut sample_size = Vec::new();
        let mut sample_flags = Vec::new();

        let mut total_sample_size = 0;
        let mut total_sample_duration = 0;

        // TODO: Apply a limit.
        for _ in 0..sample_count {

            if (flags & TrunAtom::SAMPLE_DURATION_PRESENT) != 0 {
                let duration = reader.read_be_u32()?;
                total_sample_duration += u64::from(duration);
                sample_duration.push(duration);
            }

            if (flags & TrunAtom::SAMPLE_SIZE_PRESENT) != 0 {
                let size = reader.read_be_u32()?;
                total_sample_size += u64::from(size);
                sample_size.push(size);
            }

            if (flags & TrunAtom::SAMPLE_FLAGS_PRESENT) != 0 {
                sample_flags.push(reader.read_be_u32()?);
            }

            // Ignoring composition time for now since it's a video thing...
            if (flags & TrunAtom::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT) != 0 {
                // For version 0, this is a u32.
                // For version 1, this is a i32.
                let _ = reader.read_be_u32()?;
            }
        }

        if (flags & TrunAtom::SAMPLE_COMPOSITION_TIME_OFFSETS_PRESENT) != 0 {
            warn!("ignoring sample composition time offsets.");
        }

        Ok(TrunAtom {
            header,
            flags,
            data_offset,
            sample_count,
            first_sample_flags,
            sample_duration,
            sample_size,
            sample_flags,
            total_sample_size,
            total_sample_duration,
        })
    }
}