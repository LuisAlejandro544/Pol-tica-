//! Propósito del archivo:
//! Generador modular de líderes políticos aleatorios (Presidente en ejercicio, líderes
//! de todos los partidos de oposición y terna aleatoria de Ministros de Relaciones Exteriores
//! con estadísticas diplomáticas aleatorias). Se aplica por igual al país del jugador y a
//! todas las naciones controladas por la IA.

use crate::models::{CountryState, ForeignMinister, ForeignMinisterStats, LeaderProfile};
use rand::{rngs::ChaCha8Rng, RngExt, SeedableRng};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
struct GivenName {
    name: &'static str,
    is_female: bool,
}

const HISPANIC_GIVEN_NAMES: &[GivenName] = &[
    GivenName { name: "Mateo", is_female: false },
    GivenName { name: "Santiago", is_female: false },
    GivenName { name: "Alejandro", is_female: false },
    GivenName { name: "Diego", is_female: false },
    GivenName { name: "Sebastián", is_female: false },
    GivenName { name: "Andrés", is_female: false },
    GivenName { name: "Carlos", is_female: false },
    GivenName { name: "Fernando", is_female: false },
    GivenName { name: "Gabriel", is_female: false },
    GivenName { name: "Rafael", is_female: false },
    GivenName { name: "Eduardo", is_female: false },
    GivenName { name: "Rodrigo", is_female: false },
    GivenName { name: "Joaquín", is_female: false },
    GivenName { name: "Ricardo", is_female: false },
    GivenName { name: "Martín", is_female: false },
    GivenName { name: "Héctor", is_female: false },
    GivenName { name: "Emiliano", is_female: false },
    GivenName { name: "Mauricio", is_female: false },
    GivenName { name: "Valeria", is_female: true },
    GivenName { name: "Sofía", is_female: true },
    GivenName { name: "Camila", is_female: true },
    GivenName { name: "Mariana", is_female: true },
    GivenName { name: "Lucía", is_female: true },
    GivenName { name: "Paula", is_female: true },
    GivenName { name: "Andrea", is_female: true },
    GivenName { name: "Gabriela", is_female: true },
    GivenName { name: "Claudia", is_female: true },
    GivenName { name: "Natalia", is_female: true },
    GivenName { name: "Verónica", is_female: true },
    GivenName { name: "Elena", is_female: true },
    GivenName { name: "Patricia", is_female: true },
    GivenName { name: "Beatriz", is_female: true },
    GivenName { name: "Isabel", is_female: true },
    GivenName { name: "Daniela", is_female: true },
];

const HISPANIC_SURNAMES: &[&str] = &[
    "Mendoza", "Castillo", "Salazar", "Morales", "Ortega", "Vargas", "Rojas", "Delgado",
    "Herrera", "Navarro", "Cisneros", "Guerrero", "Alvarado", "Paredes", "Velasco",
    "Cárdenas", "Figueroa", "Zambrano", "Montoya", "Aguilar", "Villalobos", "Ibarra",
    "Escobar", "Quintero", "Peñaloza", "Benítez", "Ríos", "Serrano", "Bustos", "Solís",
    "Valdivia", "Cortés", "Núñez", "Araya", "Fuentes", "Miranda", "Echeverría", "Acosta",
];

const BRAZIL_GIVEN_NAMES: &[GivenName] = &[
    GivenName { name: "Lucas", is_female: false },
    GivenName { name: "Gustavo", is_female: false },
    GivenName { name: "Henrique", is_female: false },
    GivenName { name: "Rafael", is_female: false },
    GivenName { name: "Thiago", is_female: false },
    GivenName { name: "Bruno", is_female: false },
    GivenName { name: "Marcelo", is_female: false },
    GivenName { name: "André", is_female: false },
    GivenName { name: "Felipe", is_female: false },
    GivenName { name: "Eduardo", is_female: false },
    GivenName { name: "Mariana", is_female: true },
    GivenName { name: "Beatriz", is_female: true },
    GivenName { name: "Camila", is_female: true },
    GivenName { name: "Larissa", is_female: true },
    GivenName { name: "Fernanda", is_female: true },
    GivenName { name: "Juliana", is_female: true },
    GivenName { name: "Helena", is_female: true },
    GivenName { name: "Isabela", is_female: true },
];

const BRAZIL_SURNAMES: &[&str] = &[
    "Silva", "Santos", "Oliveira", "Souza", "Pereira", "Costa", "Rodrigues", "Almeida",
    "Nunes", "Carvalho", "Ferreira", "Ribeiro", "Martins", "Araújo", "Barbosa", "Mendes",
    "Teixeira", "Cardoso", "Correia", "Monteiro",
];

const HAITI_GIVEN_NAMES: &[GivenName] = &[
    GivenName { name: "Jean-Luc", is_female: false },
    GivenName { name: "Marc-Antoine", is_female: false },
    GivenName { name: "Pierre", is_female: false },
    GivenName { name: "Frantz", is_female: false },
    GivenName { name: "Étienne", is_female: false },
    GivenName { name: "Réginald", is_female: false },
    GivenName { name: "Nadine", is_female: true },
    GivenName { name: "Fabienne", is_female: true },
    GivenName { name: "Mirlande", is_female: true },
    GivenName { name: "Chantal", is_female: true },
    GivenName { name: "Mireille", is_female: true },
    GivenName { name: "Claire", is_female: true },
];

const HAITI_SURNAMES: &[&str] = &[
    "Pierre-Louis", "Deschamps", "Alexandre", "Baptiste", "Toussaint", "Beaumont",
    "Augustin", "Celestin", "Lafontant", "Delva", "Joseph", "François",
];

const BELIZE_GIVEN_NAMES: &[GivenName] = &[
    GivenName { name: "Edward", is_female: false },
    GivenName { name: "Anthony", is_female: false },
    GivenName { name: "Michael", is_female: false },
    GivenName { name: "David", is_female: false },
    GivenName { name: "Carlos", is_female: false },
    GivenName { name: "Raymond", is_female: false },
    GivenName { name: "Sandra", is_female: true },
    GivenName { name: "Denise", is_female: true },
    GivenName { name: "Elena", is_female: true },
    GivenName { name: "Margaret", is_female: true },
    GivenName { name: "Alicia", is_female: true },
    GivenName { name: "Valerie", is_female: true },
];

const BELIZE_SURNAMES: &[&str] = &[
    "Castillo", "Young", "Barrow", "Hyde", "Musa", "Marin", "Smith", "gentle", "Williams",
    "Espat", "Hulse", "Flowers",
];

fn demonym_for_country(country_id: &str, is_female: bool) -> &'static str {
    match (country_id, is_female) {
        ("MX", true) => "Mexicana",
        ("MX", false) => "Mexicano",
        ("BZ", true) => "Beliceña",
        ("BZ", false) => "Beliceño",
        ("GT", _) => "Guatemalteca",
        ("SV", true) => "Salvadoreña",
        ("SV", false) => "Salvadoreño",
        ("HN", true) => "Hondureña",
        ("HN", false) => "Hondureño",
        ("NI", _) => "Nicaragüense",
        ("CR", _) => "Costarricense",
        ("PA", true) => "Panameña",
        ("PA", false) => "Panameño",
        ("CU", true) => "Cubana",
        ("CU", false) => "Cubano",
        ("HT", true) => "Haitiana",
        ("HT", false) => "Haitiano",
        ("DO", true) => "Dominicana",
        ("DO", false) => "Dominicano",
        ("CO", true) => "Colombiana",
        ("CO", false) => "Colombiano",
        ("VE", true) => "Venezolana",
        ("VE", false) => "Venezolano",
        ("EC", true) => "Ecuatoriana",
        ("EC", false) => "Ecuatoriano",
        ("PE", true) => "Peruana",
        ("PE", false) => "Peruano",
        ("BR", true) => "Brasileña",
        ("BR", false) => "Brasileño",
        ("BO", true) => "Boliviana",
        ("BO", false) => "Boliviano",
        ("PY", true) => "Paraguaya",
        ("PY", false) => "Paraguayo",
        ("UY", true) => "Uruguaya",
        ("UY", false) => "Uruguayo",
        ("CL", true) => "Chilena",
        ("CL", false) => "Chileno",
        ("AR", true) => "Argentina",
        ("AR", false) => "Argentino",
        _ => "Latinoamericana",
    }
}

const ALL_COUNTRY_IDS: &[&str] = &[
    "MX", "BZ", "GT", "SV", "HN", "NI", "CR", "PA", "CU", "HT", "DO", "CO", "VE", "EC", "PE",
    "BR", "BO", "PY", "UY", "CL", "AR",
];

fn binational_heritage(country_id: &str, is_female: bool, rng: &mut ChaCha8Rng) -> String {
    let local = demonym_for_country(country_id, is_female);
    let other_id = ALL_COUNTRY_IDS[rng.random_range(0..ALL_COUNTRY_IDS.len())];
    if other_id != country_id && rng.random_bool(0.55) {
        let other = demonym_for_country(other_id, is_female);
        format!("{local} (doble nac. {other})")
    } else {
        let roots = if is_female {
            ["Ítalo", "Hispano", "Luso", "Franco", "Germano", "Libanés"]
        } else {
            ["Ítalo", "Hispano", "Luso", "Franco", "Germano", "Libanés"]
        };
        let root = roots[rng.random_range(0..roots.len())];
        format!("{root}-{local}")
    }
}

fn generate_single_leader(
    country_id: &str,
    is_ruling: bool,
    rng: &mut ChaCha8Rng,
    used_names: &[String],
) -> LeaderProfile {
    let (given_pool, surname_pool) = match country_id {
        "BR" => (BRAZIL_GIVEN_NAMES, BRAZIL_SURNAMES),
        "HT" => (HAITI_GIVEN_NAMES, HAITI_SURNAMES),
        "BZ" => (BELIZE_GIVEN_NAMES, BELIZE_SURNAMES),
        _ => (HISPANIC_GIVEN_NAMES, HISPANIC_SURNAMES),
    };

    let mut chosen_name = String::new();
    let mut is_female = false;

    for _ in 0..16 {
        let gn = given_pool[rng.random_range(0..given_pool.len())];
        let s1_raw = surname_pool[rng.random_range(0..surname_pool.len())];
        let s1 = capitalize_surname(s1_raw);
        let candidate = if country_id == "HT" || country_id == "BZ" || rng.random_bool(0.45) {
            format!("{} {s1}", gn.name)
        } else {
            let mut s2_raw = surname_pool[rng.random_range(0..surname_pool.len())];
            if s2_raw == s1_raw {
                s2_raw = surname_pool[(rng.random_range(0..surname_pool.len()) + 1) % surname_pool.len()];
            }
            let s2 = capitalize_surname(s2_raw);
            format!("{} {s1} {s2}", gn.name)
        };
        if !used_names.iter().any(|existing| existing == &candidate) {
            chosen_name = candidate;
            is_female = gn.is_female;
            break;
        }
    }

    if chosen_name.is_empty() {
        let gn = given_pool[rng.random_range(0..given_pool.len())];
        let s1 = capitalize_surname(surname_pool[rng.random_range(0..surname_pool.len())]);
        chosen_name = format!("{} {s1}", gn.name);
        is_female = gn.is_female;
    }

    let age: u8 = if is_ruling {
        rng.random_range(39_u8..=71_u8)
    } else {
        rng.random_range(33_u8..=69_u8)
    };

    let nationality = if rng.random_bool(0.16) {
        binational_heritage(country_id, is_female, rng)
    } else {
        demonym_for_country(country_id, is_female).to_string()
    };

    LeaderProfile {
        full_name: chosen_name,
        age,
        nationality,
    }
}

fn capitalize_surname(s: &str) -> String {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut out = first.to_uppercase().to_string();
    out.push_str(chars.as_str());
    out
}

/// Asigna aleatoriamente Presidente y líderes de oposición (nombre, edad y nacionalidad)
/// tanto para el país del jugador como para todos los países de la IA.
pub fn assign_random_leaders_to_countries(
    countries: &mut BTreeMap<String, CountryState>,
    seed: u64,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x4C45_4144_4552_2026);
    for (cid, country) in countries.iter_mut() {
        let mut used_names = Vec::with_capacity(country.political_parties.len());
        for party in &mut country.political_parties {
            let leader = generate_single_leader(cid, party.is_ruling, &mut rng, &used_names);
            used_names.push(leader.full_name.clone());
            party.leader = leader;
        }
    }
}

/// Genera un sucesor aleatorio tras renovación electoral o retiro por edad avanzada.
pub fn generate_successor_leader(
    country_id: &str,
    is_ruling: bool,
    seed: u64,
    used_names: &[String],
) -> LeaderProfile {
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x5355_4343_4553_5301);
    generate_single_leader(country_id, is_ruling, &mut rng, used_names)
}

const MINISTER_SPECIALTIES: &[&str] = &[
    "Mediación de Paz",
    "Arquitectura de Alianzas",
    "Distensión Regional",
    "Diplomacia Bilateral",
];

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Genera una lista aleatoria de candidatos a Ministro de Relaciones Exteriores con estadísticas propias.
pub fn generate_minister_candidates(
    country_id: &str,
    seed: u64,
    next_id: &mut u64,
    count: usize,
    used_names: &[String],
) -> Vec<ForeignMinister> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0x4D49_4E49_5354_4552);
    let mut existing = used_names.to_vec();
    let mut out = Vec::with_capacity(count);

    for idx in 0..count {
        let profile = generate_single_leader(country_id, false, &mut rng, &existing);
        existing.push(profile.full_name.clone());

        let spec_idx = (idx + rng.random_range(0..MINISTER_SPECIALTIES.len()))
            % MINISTER_SPECIALTIES.len();
        let specialty = MINISTER_SPECIALTIES[spec_idx].to_string();

        // Estadísticas aleatorias con sesgo ligero por especialidad para variedad real
        let mut relation_boost_pct = rng.random_range(16.0_f64..=52.0_f64);
        let mut peace_treaty_chance_pct = rng.random_range(36.0_f64..=86.0_f64);
        let mut white_peace_chance_pct = rng.random_range(44.0_f64..=92.0_f64);
        let mut alliance_chance_pct = rng.random_range(28.0_f64..=82.0_f64);
        let mut non_aggression_chance_pct = rng.random_range(42.0_f64..=90.0_f64);

        match spec_idx {
            0 => {
                peace_treaty_chance_pct = (peace_treaty_chance_pct + 9.0).min(95.0);
                white_peace_chance_pct = (white_peace_chance_pct + 8.0).min(96.0);
            }
            1 => {
                alliance_chance_pct = (alliance_chance_pct + 10.0).min(94.0);
                non_aggression_chance_pct = (non_aggression_chance_pct + 5.0).min(95.0);
            }
            2 => {
                non_aggression_chance_pct = (non_aggression_chance_pct + 9.0).min(96.0);
                relation_boost_pct = (relation_boost_pct + 8.0).min(65.0);
            }
            _ => {
                relation_boost_pct = (relation_boost_pct + 12.0).min(68.0);
                alliance_chance_pct = (alliance_chance_pct + 5.0).min(90.0);
            }
        }

        let id = *next_id;
        *next_id = next_id.saturating_add(1);

        out.push(ForeignMinister {
            id,
            profile,
            specialty,
            stats: ForeignMinisterStats {
                relation_boost_pct: round1(relation_boost_pct),
                peace_treaty_chance_pct: round1(peace_treaty_chance_pct),
                white_peace_chance_pct: round1(white_peace_chance_pct),
                alliance_chance_pct: round1(alliance_chance_pct),
                non_aggression_chance_pct: round1(non_aggression_chance_pct),
            },
        });
    }

    out
}

/// Inicializa las ternas de candidatos a Ministro de Relaciones Exteriores por país.
/// Deja `foreign_minister = None` para que el jugador asigne manualmente a su Canciller.
pub fn assign_initial_minister_pools(
    countries: &mut BTreeMap<String, CountryState>,
    seed: u64,
    next_minister_id: &mut u64,
) {
    for (idx, (cid, country)) in countries.iter_mut().enumerate() {
        let mut used: Vec<String> = country
            .political_parties
            .iter()
            .map(|p| p.leader.full_name.clone())
            .collect();
        let country_seed = seed
            ^ ((idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
            ^ cid.bytes().fold(0_u64, |acc, b| acc.rotate_left(7) ^ (b as u64));
        let candidates = generate_minister_candidates(cid, country_seed, next_minister_id, 4, &used);
        for c in &candidates {
            used.push(c.profile.full_name.clone());
        }
        country.minister_candidates = candidates;
        country.foreign_minister = None;
    }
}
