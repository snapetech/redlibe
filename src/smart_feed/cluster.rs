use std::collections::HashMap;

// Minimal, per-page simhash clustering. Persisting clusters is optional.
pub fn title_norm(s: &str) -> String {
	let s = s.to_lowercase();
	let mut out = String::with_capacity(s.len());
	for ch in s.chars() {
		if ch.is_ascii_alphanumeric() || ch == ' ' {
			out.push(ch);
		} else {
			out.push(' ');
		}
	}
	out
		.split_whitespace()
		.filter(|w| !STOPWORDS.contains(w))
		.collect::<Vec<_>>()
		.join(" ")
}

// Very small stopword list; extend as desired.
const STOPWORDS: &[&str] = &[
	"the", "a", "an", "and", "or", "to", "of", "in", "on", "for", "with", "is", "are", "was", "were", "this", "that", "it", "as", "by",
	"from", "at", "be", "have", "has", "had",
];

pub fn simhash64(text: &str) -> u64 {
	let mut v = [0i32; 64];
	for token in text.split_whitespace() {
		let h = fxhash64(token.as_bytes());
		for i in 0..64 {
			let bit = (h >> i) & 1;
			v[i] += if bit == 1 { 1 } else { -1 };
		}
	}
	let mut out = 0u64;
	for i in 0..64 {
		if v[i] >= 0 {
			out |= 1u64 << i;
		}
	}
	out
}

fn fxhash64(bytes: &[u8]) -> u64 {
	// tiny, stable-ish: FNV-1a 64-bit
	const FNV_OFFSET: u64 = 0xcbf29ce484222325;
	const FNV_PRIME: u64 = 0x100000001b3;
	let mut h = FNV_OFFSET;
	for &b in bytes {
		h ^= b as u64;
		h = h.wrapping_mul(FNV_PRIME);
	}
	h
}

fn hamming(a: u64, b: u64) -> u32 {
	(a ^ b).count_ones()
}

#[derive(Debug, Clone)]
pub struct Cluster {
	pub id: String,
	pub members: Vec<String>, // post ids
}

// items: Vec of (post_id, title)
pub fn cluster_titles(items: &[(String, String)], max_dist: u32) -> HashMap<String, Cluster> {
	// bucket by top 16 bits of simhash to reduce comparisons
	let mut buckets: HashMap<u16, Vec<(String, u64)>> = HashMap::new();
	for (post_id, title) in items {
		let n = title_norm(title);
		let h = simhash64(&n);
		let b = (h >> 48) as u16;
		buckets.entry(b).or_default().push((post_id.clone(), h));
	}

	let mut clusters: HashMap<String, Cluster> = HashMap::new();
	let mut assigned: HashMap<String, String> = HashMap::new(); // post_id -> cluster_id

	for (_b, bucket_items) in buckets {
		for (i_id, i_h) in &bucket_items {
			if assigned.contains_key(i_id) {
				continue;
			}
			let cluster_id = format!("c{:016x}", i_h);
			let mut members = vec![i_id.clone()];
			assigned.insert(i_id.clone(), cluster_id.clone());

			for (j_id, j_h) in &bucket_items {
				if i_id == j_id {
					continue;
				}
				if assigned.contains_key(j_id) {
					continue;
				}
				if hamming(*i_h, *j_h) <= max_dist {
					members.push(j_id.clone());
					assigned.insert(j_id.clone(), cluster_id.clone());
				}
			}

			clusters.insert(cluster_id.clone(), Cluster { id: cluster_id, members });
		}
	}

	clusters
}
