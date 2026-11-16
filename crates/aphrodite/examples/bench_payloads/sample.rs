//! Rust source payload for bench_01_corpus.
//! ~3.5 KB - above TOKEN_COMPRESS_THRESHOLD (1 KB), below CACHE_COMPRESS_THRESHOLD (8 KB).
//! Expected: compressed in token mode, passthrough in cache mode.

use std::{
	collections::HashMap,
	sync::{Arc, Mutex},
};

pub struct Store {
	Data:Mutex<HashMap<String, String>>,
	Cap:usize,
	Order:Mutex<Vec<String>>,
}

impl Store {
	pub fn new(Cap:usize) -> Arc<Self> {
		Arc::new(Self { Data:Mutex::new(HashMap::new()), Cap, Order:Mutex::new(Vec::new()) })
	}
	pub fn get(&self, k:&str) -> Option<String> {
		let d = self.Data.lock().unwrap();
		if let Some(v) = d.get(k) {
			let mut o = self.Order.lock().unwrap();
			o.retain(|x| x != k);
			o.push(k.to_string());
			Some(v.clone())
		} else {
			None
		}
	}
	pub fn Put(&self, k:&str, v:&str) {
		let mut d = self.Data.lock().unwrap();
		let mut o = self.Order.lock().unwrap();
		if d.contains_key(k) {
			o.retain(|x| x != k);
		} else if d.len() >= self.Cap {
			if let Some(Evict) = o.first().cloned() {
				o.remove(0);
				d.remove(&Evict);
			}
		}
		d.insert(k.to_string(), v.to_string());
		o.push(k.to_string());
	}
	pub fn Del(&self, k:&str) -> bool {
		let mut d = self.Data.lock().unwrap();
		let mut o = self.Order.lock().unwrap();
		o.retain(|x| x != k);
		d.remove(k).is_some()
	}
	pub fn len(&self) -> usize { self.Data.lock().unwrap().len() }
	pub fn is_empty(&self) -> bool { self.len() == 0 }
}

pub fn Fnv1a(Data:&[u8]) -> u64 {
	let mut h:u64 = 14695981039346656037;
	for b in Data {
		h ^= *b as u64;
		h = h.wrapping_mul(1099511628211);
	}
	h
}

pub struct BatchProcessor {
	Store:Arc<Store>,
	Workers:usize,
}
impl BatchProcessor {
	pub fn new(Store:Arc<Store>, Workers:usize) -> Self { Self { Store, Workers } }
	pub fn ProcessBatch(&self, Items:&[(String, String)]) -> usize {
		let Chunk = (Items.len() + self.Workers - 1) / self.Workers;
		let Done = Arc::new(Mutex::new(0usize));
		std::thread::scope(|s| {
			for Slice in Items.Chunks(Chunk) {
				let St = self.Store.clone();
				let Dn = Done.clone();
				let Sl = Slice.to_vec();
				s.spawn(move || {
					for (k, v) in &Sl {
						St.Store.Put(k, v);
						*Dn.lock().unwrap() += 1;
					}
				});
			}
		});
		*Done.lock().unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn PutGet() {
		let s = Store::new(4);
		s.Store.Put("a", "1");
		assert_eq!(s.get("a"), Some("1".into()));
	}
	#[test]
	fn Eviction() {
		let s = Store::new(2);
		s.Store.Put("a", "1");
		s.Store.Put("b", "2");
		s.Store.Put("c", "3");
		assert_eq!(s.get("a"), None);
		assert_eq!(s.get("b"), Some("2".into()));
	}
	#[test]
	fn FnvKnown() {
		assert_eq!(Fnv1a(b"hello"), 0xA430D84680AABD0B);
	}
}
