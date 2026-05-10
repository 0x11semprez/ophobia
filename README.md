# Ophobia

> A research project on a privacy-first blockchain, designed from the ground up to resist a global passive network adversary.

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

The architecture is organized around three co-designed layers:

| Layer            | Goal                                                       | Tools we are studying                              |
| ---------------- | ---------------------------------------------------------- | -------------------------------------------------- |
| **Cryptography** | Hide amounts, addresses, and the transaction graph.        | Zero-knowledge proofs, ring signatures, commitments. |
| **Diffusion**    | Prevent timing and topology-based deanonymization of gossip. | Dandelion++-style protocols, batched relays, cover traffic. |
| **Network**      | Hide IP-level metadata of every node.                      | Mixnets, traffic shaping, decoy traffic.            |

Unlike previous work that bolts a privacy network (Tor, I2P) onto an existing chain, Ophobia treats network-layer privacy as a **first-class protocol concern**.

## Ethical Disclaimer

In May 2024, Alexey Pertsev, developer of Tornado Cash, was sentenced in the Netherlands to 5 years and 4 months in prison for money laundering through the Tornado Cash protocol.

This project is **strictly academic and research-oriented**:

- The full implementation will **not** be released publicly.
- Safeguards are integrated by design, including:
  - A blacklist of sanctioned addresses.
  - Optional compliance mechanisms (e.g. selective disclosure, viewing keys).
- The goal is to **explore the trade-offs between strong anonymity and regulatory compliance**, not to provide an operational tool for circumvention.

Any publications resulting from this work will follow responsible-disclosure principles and the ethical guidelines of our institution.

## Team

| Name                       | Role                  |
| -------------------------- | --------------------- |
| **Yannis Manicord**        | Cryptography Engineer |
| **Kassim Traoré-Semprez**  | Mixnet Engineer       |

## Status

Early-stage research. This repository currently hosts the project's documentation and research notes; no production code is published here.
