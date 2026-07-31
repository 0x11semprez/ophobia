# Ophobia

> A research project on a mixnet designed from the ground up to resist a global passive network adversary.

## Project History

Ophobia started as a much broader effort: a **privacy-first blockchain** co-designing the cryptographic, diffusion and network layers in one architecture. After scoping the work, we decided to **drop the blockchain and focus only on the mixnet**, the network layer that hides IP-level metadata. That layer is where existing privacy coins are weakest, and it is useful on its own, independently of any ledger.

The sections below keep the original motivation, because the research question and threat model still drive the design of the mixnet.

## Research Question

In a global context where data protection and online anonymity have become strategic concerns, **can two economic agents today perform a truly anonymous monetary transaction?**

Existing privacy coins such as **Monero** and **Zcash** offer robust answers at the *transactional* layer — hiding amounts, addresses and the transaction graph — but leave the **network layer largely exposed**. Faced with a global passive adversary capable of observing a significant fraction of internet traffic simultaneously, metadata analysis (IP addresses, propagation timing, peer topology) is enough to deanonymize a substantial share of transactions, **without breaking the underlying cryptography**.

Ophobia is a research effort to design a blockchain entirely dedicated to confidentiality, **co-designing the cryptographic, diffusion and network layers** within a single unified architecture, built from day one to resist this threat model.

## Threat Model

We assume a **Global Passive Adversary (GPA)** with the following capabilities:

- Observes a significant fraction of internet traffic in real time.
- Records IP-level metadata, packet timing and peer-to-peer topology.
- Cannot break standard cryptographic primitives.
- May correlate on-chain data with off-chain network observations.

Existing privacy coins are **not** designed against this model: they protect the ledger, not the wire.

## Research Goals

1. **Characterize** the network-layer leakage of current privacy blockchains under a GPA.
2. **Design** a unified architecture combining:
   - **Cryptographic layer** — confidential transactions, sender/receiver/amount privacy.
   - **Diffusion layer** — gossip protocols hardened against timing and topology analysis.
   - **Network layer** — mixnet-based transport providing strong unlinkability of broadcast traffic.
3. **Evaluate** the trade-offs between strong anonymity, latency, throughput and regulatory compliance.
4. **Explore** optional compliance hooks (selective disclosure, sanction-list filtering) compatible with strong default privacy.

## Approach

The architecture is organized around three co-designed layers — **Cryptography** (hiding amounts, addresses and the transaction graph via zero-knowledge proofs, ring signatures and commitments), **Diffusion** (hardening gossip against timing and topology analysis with Dandelion++-style protocols, batched relays and cover traffic) and **Network** (hiding IP-level metadata via mixnets, traffic shaping and decoy traffic).

Unlike previous work that bolts a privacy network (Tor, I2P) onto an existing chain, Ophobia treats network-layer privacy as a **first-class protocol concern**.

A detailed research thesis covering the threat model, architecture and trade-offs will be linked here.

## Ethical Disclaimer

This project is **strictly academic and research-oriented**. The full implementation will **not** be released publicly, safeguards (sanctioned-address blacklist, selective disclosure, viewing keys) are integrated by design, and the goal is to **explore the trade-offs between strong anonymity and regulatory compliance** — not to provide an operational tool for circumvention. Any publications will follow responsible-disclosure principles and the ethical guidelines of our institution.

## Team

- [@0x7manny](https://github.com/0x7manny) — Cryptography Engineer
- [@0x11semprez](https://github.com/0x11semprez) — Mixnet Engineer

## Repository Layout

- `cmd/mixnet/` — entry point (`main.go`).
- `internal/` — private packages (`cryptography/`, `network/`, `informations/`).
- `tests/` — tests.

## Status

Early-stage research. Scope is now **the mixnet only**; the blockchain, cryptographic ledger and diffusion layers are out of scope.
