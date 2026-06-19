// Copyright (c) 2026 Matt Jones. All rights reserved.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Safety Element Out Of Context (SEOOC) assumptions for rust-LIN.
//!
//! rust-LIN is compliant with ISO 26262 ASIL-B at the SEOOC level.
//! The remaining ASIL-B obligations are allocated to the integrating system.
//!
//! Assumption requirements (REQ-SEOOC-002, 003, 007, 008, 009) are
//! obligations on the system that uses rust-LIN, not on rust-LIN itself.
//! They are recorded here so that rust-FuSa can include them in the safety
//! case and traceability report.
//!
//! Integration requirements (REQ-SEOOC-004, 005, 006) are verified by
//! the integration tests in `tests/integration_test.rs`.

//fusa:req REQ-SEOOC-001
//fusa:req REQ-SEOOC-002
//fusa:req REQ-SEOOC-003
//fusa:req REQ-SEOOC-004
//fusa:req REQ-SEOOC-005
//fusa:req REQ-SEOOC-006
//fusa:req REQ-SEOOC-007
//fusa:req REQ-SEOOC-008
//fusa:req REQ-SEOOC-009

// This module is declaration-only; the integration tests provide evidence.
