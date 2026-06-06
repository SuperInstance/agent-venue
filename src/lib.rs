//! # agent-venue
//!
//! Environmental acoustics affecting agent communication.
//!
//! Agents in "small rooms" (low latency, high bandwidth) behave differently
//! than agents in "cathedrals" (high latency, intermittent connectivity).
//! This crate models the communication environment as a *venue* with acoustic
//! properties that shape how agents should coordinate.

use std::collections::HashMap;
use std::time::Duration;

// ── Acoustic profile ───────────────────────────────────────────────────────

/// The acoustic fingerprint of a venue.
///
/// Analogous to room acoustics: latency is reverb, bandwidth is clarity,
/// reliability is how often the sound cuts out.
#[derive(Debug, Clone)]
pub struct AcousticProfile {
    /// Round-trip latency as a Duration.
    pub latency: Duration,
    /// Effective bandwidth in bytes/second.
    pub bandwidth_bps: u64,
    /// Message delivery reliability: 0.0 (nothing arrives) to 1.0 (perfect).
    pub reliability: f64,
    /// Maximum concurrent connections the venue supports.
    pub max_connections: usize,
}

impl AcousticProfile {
    /// A tight, low-latency environment (e.g. same datacenter).
    pub fn small_room() -> Self {
        Self {
            latency: Duration::from_millis(5),
            bandwidth_bps: 10_000_000,
            reliability: 0.999,
            max_connections: 50,
        }
    }

    /// A moderate environment (e.g. same region).
    pub fn concert_hall() -> Self {
        Self {
            latency: Duration::from_millis(50),
            bandwidth_bps: 1_000_000,
            reliability: 0.99,
            max_connections: 200,
        }
    }

    /// A high-latency, reverberant environment (e.g. cross-globe, satellite).
    pub fn cathedral() -> Self {
        Self {
            latency: Duration::from_millis(500),
            bandwidth_bps: 100_000,
            reliability: 0.9,
            max_connections: 500,
        }
    }

    /// An extremely hostile environment (e.g. disaster zone, deep space).
    pub fn cavern() -> Self {
        Self {
            latency: Duration::from_secs(5),
            bandwidth_bps: 10_000,
            reliability: 0.5,
            max_connections: 20,
        }
    }

    /// Classify the venue into a named tier.
    pub fn tier(&self) -> VenueTier {
        let lat_ms = self.latency.as_millis() as u64;
        if lat_ms <= 10 && self.reliability >= 0.99 {
            VenueTier::UltraLowLatency
        } else if lat_ms <= 100 && self.reliability >= 0.95 {
            VenueTier::LowLatency
        } else if lat_ms <= 1000 && self.reliability >= 0.8 {
            VenueTier::MediumLatency
        } else {
            VenueTier::HighLatency
        }
    }

    /// Compute a composite "quality" score 0.0–1.0.
    pub fn quality_score(&self) -> f64 {
        let lat_score = 1.0 - (self.latency.as_secs_f64().ln_1p() / 5.0_f64.ln_1p()).min(1.0);
        let bw_score = (self.bandwidth_bps as f64).ln_1p() / (100_000_000_f64).ln_1p();
        let rel_score = self.reliability;
        (lat_score * 0.3 + bw_score * 0.3 + rel_score * 0.4).min(1.0).max(0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenueTier {
    UltraLowLatency,
    LowLatency,
    MediumLatency,
    HighLatency,
}

// ── Delay profile ──────────────────────────────────────────────────────────

/// Models communication delays as "room echo".
///
/// In real acoustics, early reflections arrive first, then a decaying tail
/// of reverberation. Here, messages may arrive with variable delays.
#[derive(Debug, Clone)]
pub struct DelayProfile {
    /// Base latency (the "direct sound").
    pub base_delay: Duration,
    /// Additional jitter as a Duration range.
    pub jitter_range: (Duration, Duration),
    /// Probability of a "lost" message (analogue: sound absorbed).
    pub loss_probability: f64,
    /// Simulated packet reordering probability.
    pub reorder_probability: f64,
}

impl DelayProfile {
    pub fn from_acoustic(profile: &AcousticProfile) -> Self {
        let base = profile.latency;
        let jitter = base / 10;
        Self {
            base_delay: base,
            jitter_range: (Duration::ZERO, jitter),
            loss_probability: 1.0 - profile.reliability,
            reorder_probability: if base > Duration::from_millis(100) {
                0.05
            } else {
                0.01
            },
        }
    }

    /// Expected worst-case delay.
    pub fn worst_case(&self) -> Duration {
        self.base_delay + self.jitter_range.1
    }

    /// Expected average delay.
    pub fn average(&self) -> Duration {
        let avg_jitter =
            (self.jitter_range.0.as_nanos() + self.jitter_range.1.as_nanos()) / 2;
        self.base_delay + Duration::from_nanos(avg_jitter as u64)
    }
}

// ── Noise floor ────────────────────────────────────────────────────────────

/// Background interference in the communication channel.
///
/// Noise corrupts messages, causes misinterpretation, and forces agents to
/// repeat themselves (reducing effective bandwidth).
#[derive(Debug, Clone)]
pub struct NoiseFloor {
    /// 0.0 = perfectly clean channel, 1.0 = pure noise.
    pub level: f64,
    /// Named sources of noise.
    pub sources: Vec<NoiseSource>,
}

#[derive(Debug, Clone)]
pub struct NoiseSource {
    pub name: String,
    pub intensity: f64,
    pub intermittent: bool,
}

impl NoiseFloor {
    pub fn silence() -> Self {
        Self {
            level: 0.0,
            sources: Vec::new(),
        }
    }

    pub fn moderate() -> Self {
        Self {
            level: 0.3,
            sources: vec![
                NoiseSource {
                    name: "background_traffic".into(),
                    intensity: 0.2,
                    intermittent: true,
                },
            ],
        }
    }

    pub fn heavy() -> Self {
        Self {
            level: 0.7,
            sources: vec![
                NoiseSource {
                    name: "cross_talk".into(),
                    intensity: 0.4,
                    intermittent: false,
                },
                NoiseSource {
                    name: "hardware_faults".into(),
                    intensity: 0.3,
                    intermittent: true,
                },
            ],
        }
    }

    /// Effective signal-to-noise ratio (higher is better).
    pub fn snr(&self) -> f64 {
        if self.level >= 1.0 {
            return 0.0;
        }
        (1.0 - self.level) / self.level.max(0.001)
    }

    /// Probability a message survives the noise intact.
    pub fn message_integrity(&self) -> f64 {
        (1.0 - self.level).max(0.0)
    }
}

// ── Venue ──────────────────────────────────────────────────────────────────

/// A named communication environment.
#[derive(Debug, Clone)]
pub struct Venue {
    pub name: String,
    pub acoustic: AcousticProfile,
    pub noise: NoiseFloor,
    pub delay: DelayProfile,
    pub tags: Vec<String>,
}

impl Venue {
    pub fn new(name: &str, acoustic: AcousticProfile) -> Self {
        let delay = DelayProfile::from_acoustic(&acoustic);
        Self {
            name: name.to_string(),
            acoustic,
            noise: NoiseFloor::silence(),
            delay,
            tags: Vec::new(),
        }
    }

    pub fn with_noise(mut self, noise: NoiseFloor) -> Self {
        self.noise = noise;
        self
    }

    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.tags = tags.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Overall venue quality combining acoustics and noise.
    pub fn overall_quality(&self) -> f64 {
        let aq = self.acoustic.quality_score();
        let nq = self.noise.message_integrity();
        (aq * 0.6 + nq * 0.4).min(1.0).max(0.0)
    }
}

// ── Venue simulation ───────────────────────────────────────────────────────

/// Simulates message delivery under venue conditions.
#[derive(Debug, Clone)]
pub struct SimulatedMessage {
    pub id: usize,
    pub sent_at: Duration,
    pub delivered_at: Option<Duration>,
    pub corrupted: bool,
    pub lost: bool,
}

#[derive(Debug, Clone)]
pub struct VenueSimulation {
    pub venue: Venue,
    pub messages: Vec<SimulatedMessage>,
    rng_seed: u64,
}

impl VenueSimulation {
    pub fn new(venue: Venue, rng_seed: u64) -> Self {
        Self {
            venue,
            messages: Vec::new(),
            rng_seed,
        }
    }

    /// Simple deterministic PRNG (xorshift) for reproducibility.
    fn next_random(&mut self) -> f64 {
        self.rng_seed ^= self.rng_seed << 13;
        self.rng_seed ^= self.rng_seed >> 7;
        self.rng_seed ^= self.rng_seed << 17;
        (self.rng_seed as f64) / (u64::MAX as f64)
    }

    /// Simulate sending `count` messages at 1ms intervals starting from `t0`.
    pub fn simulate(&mut self, count: usize, t0: Duration) -> &Vec<SimulatedMessage> {
        self.messages.clear();
        let mut t = t0;
        for i in 0..count {
            let rand1 = self.next_random();
            let rand2 = self.next_random();
            let rand3 = self.next_random();

            let lost = rand1 < self.venue.delay.loss_probability;
            let corrupted = !lost && rand2 < self.venue.noise.level;

            let jitter_nanos = if !lost {
                let range = self.venue.delay.jitter_range;
                let base_ns = range.0.as_nanos() as f64;
                let span_ns = (range.1.as_nanos() as f64) - base_ns;
                (base_ns + rand3 * span_ns) as u64
            } else {
                0
            };

            let delivery = if !lost {
                Some(t + self.venue.delay.base_delay + Duration::from_nanos(jitter_nanos))
            } else {
                None
            };

            self.messages.push(SimulatedMessage {
                id: i,
                sent_at: t,
                delivered_at: delivery,
                corrupted,
                lost,
            });

            t += Duration::from_millis(1);
        }
        &self.messages
    }

    /// Stats from the last simulation run.
    pub fn stats(&self) -> SimulationStats {
        let total = self.messages.len();
        let delivered = self.messages.iter().filter(|m| !m.lost).count();
        let lost = self.messages.iter().filter(|m| m.lost).count();
        let corrupted = self.messages.iter().filter(|m| m.corrupted).count();

        let delays: Vec<Duration> = self
            .messages
            .iter()
            .filter_map(|m| {
                m.delivered_at.map(|d| d.saturating_sub(m.sent_at))
            })
            .collect();

        let avg_delay = if delays.is_empty() {
            Duration::ZERO
        } else {
            let sum: Duration = delays.iter().sum();
            sum / delays.len() as u32
        };
        let max_delay = delays.iter().max().copied().unwrap_or(Duration::ZERO);
        let min_delay = delays.iter().min().copied().unwrap_or(Duration::ZERO);

        SimulationStats {
            total,
            delivered,
            lost,
            corrupted,
            avg_delay,
            min_delay,
            max_delay,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimulationStats {
    pub total: usize,
    pub delivered: usize,
    pub lost: usize,
    pub corrupted: usize,
    pub avg_delay: Duration,
    pub min_delay: Duration,
    pub max_delay: Duration,
}

// ── Venue adaptation ───────────────────────────────────────────────────────

/// Strategies agents can use to adapt to venue conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptationStrategy {
    /// Send messages multiple times to combat loss.
    Retransmit { repeats: usize },
    /// Reduce message size to fit bandwidth.
    Compress { level: u8 },
    /// Batch messages and send less frequently.
    Batch { interval_ms: u64 },
    /// Fall back to simpler protocols.
    Simplify,
    /// No adaptation needed — venue is fine.
    None,
}

/// Recommends an adaptation strategy based on venue conditions.
#[derive(Debug, Clone)]
pub struct VenueAdaptation {
    pub venue_name: String,
    pub recommended: AdaptationStrategy,
    pub reason: String,
}

impl VenueAdaptation {
    pub fn recommend(venue: &Venue) -> Self {
        let profile = &venue.acoustic;
        let noise = &venue.noise;
        let delay = &venue.delay;

        let lat_ms = profile.latency.as_millis();

        let (strategy, reason) = if profile.reliability < 0.7 {
            (
                AdaptationStrategy::Retransmit { repeats: 3 },
                format!(
                    "Reliability {:.1}% is dangerously low — retransmit everything 3x",
                    profile.reliability * 100.0
                ),
            )
        } else if noise.level > 0.5 {
            (
                AdaptationStrategy::Compress { level: 9 },
                format!(
                    "Noise floor at {:.1}% — max compression to preserve signal",
                    noise.level * 100.0
                ),
            )
        } else if lat_ms > 1000 {
            (
                AdaptationStrategy::Batch { interval_ms: 500 },
                format!(
                    "Latency {}ms — batch messages every 500ms to reduce round-trips",
                    lat_ms
                ),
            )
        } else if lat_ms > 100 || profile.reliability < 0.95 {
            (
                AdaptationStrategy::Simplify,
                "Moderate latency or reliability — use simpler protocols".into(),
            )
        } else {
            (
                AdaptationStrategy::None,
                "Venue conditions are excellent — no adaptation needed".into(),
            )
        };

        VenueAdaptation {
            venue_name: venue.name.clone(),
            recommended: strategy,
            reason,
        }
    }

    /// Generate a full adaptation plan comparing multiple venues.
    pub fn compare_venues(venues: &[Venue]) -> Vec<Self> {
        venues.iter().map(Self::recommend).collect()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Acoustic profile tests ─────────────────────────────────────────

    #[test]
    fn small_room_profile() {
        let p = AcousticProfile::small_room();
        assert!(p.latency < Duration::from_millis(20));
        assert!(p.reliability > 0.99);
        assert_eq!(p.tier(), VenueTier::UltraLowLatency);
    }

    #[test]
    fn cathedral_profile() {
        let p = AcousticProfile::cathedral();
        assert!(p.latency >= Duration::from_millis(100));
        assert!(p.bandwidth_bps < 1_000_000);
    }

    #[test]
    fn quality_score_ordering() {
        let small = AcousticProfile::small_room();
        let hall = AcousticProfile::concert_hall();
        let cath = AcousticProfile::cathedral();
        let cav = AcousticProfile::cavern();

        assert!(small.quality_score() > hall.quality_score());
        assert!(hall.quality_score() > cath.quality_score());
        assert!(cath.quality_score() > cav.quality_score());
    }

    #[test]
    fn tier_classification() {
        assert_eq!(AcousticProfile::small_room().tier(), VenueTier::UltraLowLatency);
        assert_eq!(AcousticProfile::concert_hall().tier(), VenueTier::LowLatency);
        assert_eq!(AcousticProfile::cathedral().tier(), VenueTier::MediumLatency);
        assert_eq!(AcousticProfile::cavern().tier(), VenueTier::HighLatency);
    }

    // ── Delay profile tests ────────────────────────────────────────────

    #[test]
    fn delay_from_acoustic() {
        let p = AcousticProfile::concert_hall();
        let d = DelayProfile::from_acoustic(&p);
        assert_eq!(d.base_delay, p.latency);
        assert!(d.loss_probability > 0.0);
    }

    #[test]
    fn worst_case_exceeds_base() {
        let d = DelayProfile::from_acoustic(&AcousticProfile::cathedral());
        assert!(d.worst_case() >= d.base_delay);
    }

    #[test]
    fn average_within_range() {
        let d = DelayProfile::from_acoustic(&AcousticProfile::concert_hall());
        assert!(d.average() >= d.base_delay);
        assert!(d.average() <= d.worst_case());
    }

    // ── Noise floor tests ──────────────────────────────────────────────

    #[test]
    fn silence_noise() {
        let n = NoiseFloor::silence();
        assert_eq!(n.level, 0.0);
        assert!(n.snr() > 100.0);
        assert_eq!(n.message_integrity(), 1.0);
    }

    #[test]
    fn heavy_noise() {
        let n = NoiseFloor::heavy();
        assert!(n.level > 0.5);
        assert!(n.message_integrity() < 0.5);
        assert!(!n.sources.is_empty());
    }

    #[test]
    fn snr_decreases_with_noise() {
        let sil = NoiseFloor::silence();
        let mod_n = NoiseFloor::moderate();
        let hvy = NoiseFloor::heavy();
        assert!(sil.snr() > mod_n.snr());
        assert!(mod_n.snr() > hvy.snr());
    }

    // ── Venue tests ────────────────────────────────────────────────────

    #[test]
    fn venue_creation() {
        let v = Venue::new("test-hall", AcousticProfile::concert_hall())
            .with_noise(NoiseFloor::moderate())
            .with_tags(&["regional", "tier1"]);
        assert_eq!(v.name, "test-hall");
        assert_eq!(v.tags.len(), 2);
    }

    #[test]
    fn venue_overall_quality() {
        let good = Venue::new("good", AcousticProfile::small_room());
        let bad = Venue::new("bad", AcousticProfile::cavern())
            .with_noise(NoiseFloor::heavy());
        assert!(good.overall_quality() > bad.overall_quality());
    }

    #[test]
    fn noise_hurts_quality() {
        let clean = Venue::new("clean", AcousticProfile::concert_hall());
        let noisy = Venue::new("noisy", AcousticProfile::concert_hall())
            .with_noise(NoiseFloor::heavy());
        assert!(clean.overall_quality() > noisy.overall_quality());
    }

    // ── Simulation tests ───────────────────────────────────────────────

    #[test]
    fn simulation_perfect_venue() {
        let venue = Venue::new("perfect", AcousticProfile::small_room());
        let mut sim = VenueSimulation::new(venue, 42);
        sim.simulate(100, Duration::ZERO);
        let stats = sim.stats();
        assert_eq!(stats.total, 100);
        // Small room has 0.999 reliability → should lose very few.
        assert!(stats.lost < 5);
        assert!(stats.delivered > 90);
    }

    #[test]
    fn simulation_lossy_venue() {
        let venue = Venue::new("cavern", AcousticProfile::cavern())
            .with_noise(NoiseFloor::heavy());
        let mut sim = VenueSimulation::new(venue, 123);
        sim.simulate(1000, Duration::ZERO);
        let stats = sim.stats();
        // Cavern: reliability 0.5, heavy noise → significant loss.
        assert!(stats.lost > 200);
    }

    #[test]
    fn simulation_deterministic() {
        let venue = Venue::new("test", AcousticProfile::concert_hall());
        let mut sim1 = VenueSimulation::new(venue.clone(), 999);
        let mut sim2 = VenueSimulation::new(venue, 999);
        sim1.simulate(50, Duration::ZERO);
        sim2.simulate(50, Duration::ZERO);
        assert_eq!(sim1.stats().lost, sim2.stats().lost);
        assert_eq!(sim1.stats().corrupted, sim2.stats().corrupted);
    }

    #[test]
    fn simulation_delay_stats() {
        let venue = Venue::new("hall", AcousticProfile::concert_hall());
        let mut sim = VenueSimulation::new(venue, 77);
        sim.simulate(100, Duration::ZERO);
        let stats = sim.stats();
        assert!(stats.max_delay >= stats.min_delay);
        assert!(stats.avg_delay >= Duration::ZERO);
    }

    // ── Adaptation tests ───────────────────────────────────────────────

    #[test]
    fn adaptation_small_room_none() {
        let v = Venue::new("dc-1", AcousticProfile::small_room());
        let a = VenueAdaptation::recommend(&v);
        assert_eq!(a.recommended, AdaptationStrategy::None);
    }

    #[test]
    fn adaptation_cathedral_batch_or_simplify() {
        let v = Venue::new("satellite", AcousticProfile::cathedral());
        let a = VenueAdaptation::recommend(&v);
        match a.recommended {
            AdaptationStrategy::Batch { .. } | AdaptationStrategy::Simplify => {}
            other => panic!("Expected Batch or Simplify, got {other:?}"),
        }
    }

    #[test]
    fn adaptation_unreliable_retransmit() {
        let v = Venue::new(
            "dead-zone",
            AcousticProfile {
                latency: Duration::from_millis(10),
                bandwidth_bps: 1_000_000,
                reliability: 0.5,
                max_connections: 10,
            },
        );
        let a = VenueAdaptation::recommend(&v);
        assert!(matches!(a.recommended, AdaptationStrategy::Retransmit { .. }));
    }

    #[test]
    fn adaptation_compare_venues() {
        let venues = vec![
            Venue::new("good", AcousticProfile::small_room()),
            Venue::new("bad", AcousticProfile::cavern()),
        ];
        let plans = VenueAdaptation::compare_venues(&venues);
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].recommended, AdaptationStrategy::None);
        assert!(plans[1].recommended != AdaptationStrategy::None);
    }

    #[test]
    fn adaptation_reason_populated() {
        let v = Venue::new("test", AcousticProfile::concert_hall());
        let a = VenueAdaptation::recommend(&v);
        assert!(!a.reason.is_empty());
        assert_eq!(a.venue_name, "test");
    }
}
