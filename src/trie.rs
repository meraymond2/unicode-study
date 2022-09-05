use crate::trie::TrieMatch::PartialMatch;

// TODO: Incomplete implementation of an AdaptiveTrie. It should use arrays for
// the nodes that have 255 keys. As it is, it's still fast enough to do
// the partial matches to test the collation algorithm.
pub struct Trie<T: Clone> {
    keys: Vec<u8>,
    nodes: Vec<Trie<T>>,
    val: Option<T>,
}

#[derive(Debug, PartialEq)]
pub enum TrieMatch<T> {
    Match(T),
    PartialMatch,
    NoMatch,
}

impl<T: Clone> Trie<T> {
    pub fn from_kvs(pairs: Vec<(Vec<u32>, T)>) -> Self {
        let mut root = Trie::empty();
        for (k, v) in pairs {
            let key = key_chain(&k);
            let mut node: &mut Trie<T> = &mut root;
            for part in key.iter() {
                if let Some(idx) = node.keys.iter_mut().position(|x| x == part) {
                    node = &mut node.nodes[idx];
                } else {
                    node.keys.push(*part);
                    node.nodes.push(Trie::empty());
                    let new_len = node.nodes.len();
                    node = &mut node.nodes[new_len - 1];
                }
            }
            node.val = Some(v);
        }
        root
    }

    pub fn get(&self, k: &Vec<u32>) -> TrieMatch<T> {
        let key = key_chain(k);
        let mut node = self;
        for part in key.iter() {
            if let Some(idx) = node.keys.iter().position(|x| x == part) {
                node = &node.nodes[idx];
            } else {
                return TrieMatch::NoMatch;
            }
        }
        match &node.val {
            None => PartialMatch,
            Some(val) => TrieMatch::Match(val.clone()),
        }
    }

    fn empty() -> Self {
        Trie {
            keys: Vec::new(),
            nodes: Vec::new(),
            val: None,
        }
    }
}

fn key_chain(k: &Vec<u32>) -> Vec<u8> {
    k.iter().fold(Vec::new(), |mut acc, n| {
        acc.extend(n.to_ne_bytes());
        acc
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_get() {
        let trie = Trie::from_kvs(vec![(vec![0x0, 0xFF], "Cas"), (vec![0xABC, 0xDEF], "Luna")]);
        assert_eq!(trie.get(&vec![0x0, 0xFF]), TrieMatch::Match("Cas"));
        assert_eq!(trie.get(&vec![0xABC, 0xDEF]), TrieMatch::Match("Luna"));

        assert_eq!(trie.get(&vec![0x0]), TrieMatch::PartialMatch);
        assert_eq!(trie.get(&vec![0xABC]), TrieMatch::PartialMatch);

        assert_eq!(trie.get(&vec![0xDEF]), TrieMatch::NoMatch);
    }
}
