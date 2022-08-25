struct NodeVar<T: Clone> {
    keys: Vec<u8>,
    nodes: Vec<NodeVar<T>>,
    val: Option<T>,
}

impl<T: Clone> NodeVar<T> {
    pub fn from_kvs(pairs: Vec<(Vec<u32>, T)>) -> Self {
        let mut root = NodeVar::empty();
        for (k, v) in pairs {
            let key = key_chain(k);
            let mut node: &mut NodeVar<T> = &mut root;
            for part in key.iter() {
                if let Some(idx) = node.keys.iter_mut().position(|x| x == part) {
                    node = &mut node.nodes[idx];
                } else {
                    node.keys.push(*part);
                    node.nodes.push(NodeVar::empty());
                    let new_len = node.nodes.len();
                    node = &mut node.nodes[new_len - 1];
                }
            }
            node.val = Some(v);
        }
        root
    }

    pub fn get(&self, k: Vec<u32>) -> Option<T> {
        let key = key_chain(k);
        let mut node = self;
        for part in key.iter() {
            if let Some(idx) = node.keys.iter().position(|x| x == part) {
                node = &node.nodes[idx];
            } else {
                return None;
            }
        }
        node.val.clone()
    }

    fn empty() -> Self {
        NodeVar {
            keys: Vec::new(),
            nodes: Vec::new(),
            val: None,
        }
    }
}

fn key_chain(k: Vec<u32>) -> Vec<u8> {
    k.iter().fold(Vec::new(), |mut acc, n| {
        acc.extend(n.to_ne_bytes());
        acc
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get() {
        let trie = NodeVar::from_kvs(vec![(vec![0x0, 0xFF], "Cas"), (vec![0xABC, 0xDEF], "Luna")]);
        assert_eq!(trie.get(vec![0x0, 0xFF]), Some("Cas"));
        assert_eq!(trie.get(vec![0xABC, 0xDEF]), Some("Luna"));
    }
}
