# agent-venue

**Environmental acoustics affecting agent communication.**

Agents in "small rooms" (same datacenter, sub-millisecond latency) behave
differently than agents in "cathedrals" (cross-globe, satellite relay, half-
second round trips). This crate models the communication environment as a
*venue* with acoustic properties that shape how agents coordinate, adapt, and
perform.

## Core Concepts

### Acoustic Profile

An `AcousticProfile` is the acoustic fingerprint of a venue, mapping directly
from room acoustics:

| Acoustic Property    | Systems Analogue                        |
|----------------------|-----------------------------------------|
| Latency              | Reverb — how long the echo takes        |
| Bandwidth            | Clarity — how much detail comes through |
| Reliability          | Consistency — does the sound cut out?   |
| Max connections      | Room capacity                           |

Preset venues:

- **Small room** — same rack, sub-10ms, 99.9% reliable
- **Concert hall** — same region, ~50ms, 99% reliable
- **Cathedral** — cross-globe, ~500ms, 90% reliable
- **Cavern** — hostile environment, 5s latency, 50% reliable

Each profile has a `quality_score()` (0.0–1.0 composite) and a `tier()`
classification (UltraLowLatency through HighLatency).

### Delay Profile

`DelayProfile` models communication delays as room echo. In real acoustics,
you hear the direct sound first, then early reflections, then a decaying tail.
Here, messages arrive with a base delay plus jitter, and some are lost entirely.

The delay profile is derived automatically from the acoustic profile but can
be customised for testing specific scenarios.

### Noise Floor

`NoiseFloor` represents background interference in the communication channel.
Noise corrupts messages, causes misinterpretation, and forces agents to repeat
themselves — reducing effective bandwidth.

Key metrics:
- **SNR** (signal-to-noise ratio) — higher is better
- **Message integrity** — probability a message survives intact

Noise sources can be named and intermittent, allowing fine-grained simulation
of real-world conditions (cross-talk, hardware faults, network congestion).

### Venue

A `Venue` bundles an acoustic profile, noise floor, and delay profile into a
named environment. Venues can be tagged for filtering and compared on overall
quality.

### Venue Simulation

`VenueSimulation` models message delivery under venue conditions using a
deterministic PRNG for reproducibility. Send N messages and get back:

- How many were delivered, lost, or corrupted
- Delay statistics (average, min, max)
- Full message-by-message log for debugging

The simulation is deterministic — same seed produces identical results — making
it useful for regression testing agent behaviour across venue changes.

### Venue Adaptation

`VenueAdaptation` recommends a strategy based on venue conditions:

| Strategy        | When                            |
|-----------------|---------------------------------|
| Retransmit (3x) | Reliability < 70%               |
| Compress (max)  | Noise floor > 50%               |
| Batch (500ms)   | Latency > 1 second              |
| Simplify        | Moderate latency or reliability |
| None            | Conditions are excellent        |

## Usage

```rust
use agent_venue::*;
use std::time::Duration;

// Create a venue.
let venue = Venue::new("eu-west-satellite", AcousticProfile::cathedral())
    .with_noise(NoiseFloor::moderate());

// Simulate 100 messages.
let mut sim = VenueSimulation::new(venue.clone(), 42);
sim.simulate(100, Duration::ZERO);
let stats = sim.stats();
println!("Delivered: {}/{}", stats.delivered, stats.total);
println!("Avg delay: {:?}", stats.avg_delay);

// Get adaptation advice.
let adaptation = VenueAdaptation::recommend(&venue);
println!("Strategy: {:?} — {}", adaptation.recommended, adaptation.reason);

// Compare multiple venues.
let venues = vec![
    Venue::new("dc-1", AcousticProfile::small_room()),
    Venue::new("sat-1", AcousticProfile::cavern()),
];
let plans = VenueAdaptation::compare_venues(&venues);
for plan in &plans {
    println!("{}: {:?}", plan.venue_name, plan.recommended);
}
```

## Why Acoustics?

The metaphor is precise and useful:

- **Reverb** is latency. Long reverb means you hear yourself (echo messages)
  and can't tell when your message was actually "heard."
- **Noise** is interference. In a noisy room, you speak louder (retransmit),
  repeat yourself (redundancy), or use simpler words (protocol downgrade).
- **Room size** is network scale. Small rooms need less coordination. Large
  rooms need section leaders, visual cues, and agreed-upon protocols.
- **Audience absorption** is load. A full room absorbs more energy than an
  empty one — more agents means more message consumption and less available
  bandwidth per agent.

Understanding venue acoustics lets you *adapt your agents* rather than hoping
the network will behave. A choir that sounds great in a concert hall may be
unbearable in a cathedral — unless the director adjusts tempo, dynamics, and
phrasing. Same for distributed systems.

## License

MIT
