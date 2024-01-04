use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct CountMap<K: Hash + Eq> {
    m: HashMap<K, usize>,
}

impl<K: Hash + Eq> Default for CountMap<K> {
    fn default() -> Self {
        Self {
            m: HashMap::default(),
        }
    }
}

impl<K> CountMap<K>
where
    K: Hash + Eq,
{
    pub fn get<O>(&mut self, key: &O) -> usize
    where
        K: Borrow<O>,
        O: Hash + Eq + ToOwned<Owned = K> + ?Sized,
    {
        self.m.get(key).copied().unwrap_or(0)
    }

    pub fn get_mut<O>(&mut self, key: &O) -> &mut usize
    where
        K: Borrow<O>,
        O: Hash + Eq + ToOwned<Owned = K>,
    {
        self.m.entry(key.to_owned()).or_insert(0)
    }

    pub fn inc<O>(&mut self, key: &O)
    where
        K: Borrow<O>,
        O: Hash + Eq + ToOwned<Owned = K> + ?Sized,
    {
        if let Some(count) = self.m.get_mut(key) {
            *count += 1;
        } else {
            self.m.insert(key.to_owned(), 1);
        }
    }

    pub fn dec<O>(&mut self, key: &O)
    where
        K: Borrow<O>,
        O: Hash + Eq + ToOwned<Owned = K> + ?Sized,
    {
        if let Some(count) = self.m.get_mut(key) {
            *count -= 1;
        } else {
            self.m.insert(key.to_owned(), 1);
        }
    }

    pub fn count_nonzero(&self) -> usize {
        self.m.values().filter(|c| **c > 0).count()
    }

    pub fn iter_nonzero<O>(&self) -> impl Iterator<Item = (&O, &usize)>
    where
        K: Borrow<O>,
        O: Hash + Eq + ToOwned<Owned = K> + ?Sized,
    {
        self.m
            .iter()
            .filter_map(|(k, c)| if *c > 0 { Some((k.borrow(), c)) } else { None })
    }

    pub fn clear(&mut self) {
        self.m.clear();
    }
}

#[cfg(test)]
mod test {
    use crate::count_map::CountMap;

    #[test]
    fn basic_test() {
        let mut m = CountMap::default();
        m.inc("a");
        m.inc("a");
        m.inc("b");
        m.inc("b");
        m.inc("c");

        m.dec("a");
        m.dec("c");

        let _r: Vec<(&str, &usize)> = m.iter_nonzero().collect::<Vec<_>>();
        let counts = ["a", "b", "c"].map(|k| m.get(k));
        assert_eq!(counts, [1, 2, 0]);
    }
}
