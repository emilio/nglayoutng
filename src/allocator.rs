#[derive(Debug)]
pub struct Allocator<T> {
    nodes: Vec<Option<T>>,
    free_list: Vec<usize>,
    len: usize,
}

impl<T> Default for Allocator<T> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            free_list: Vec::new(),
            len: 0,
        }
    }
}

impl<T> Allocator<T> {
    pub fn allocate(&mut self, item: T) -> usize {
        self.len += 1;

        if let Some(free) = self.free_list.pop() {
            assert!(self.nodes[free].is_none());
            self.nodes[free] = Some(item);
            return free;
        }

        self.nodes.push(Some(item));
        self.nodes.len() - 1
    }

    pub fn deallocate(&mut self, id: usize) -> T {
        let node = self.nodes[id].take().expect("Deallocating free item");
        self.free_list.push(id);
        self.len -= 1;
        node
    }

    pub fn get(&self, id: usize) -> Option<&T> {
        self.nodes[id].as_ref()
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut T> {
        self.nodes[id].as_mut()
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T> ::std::ops::Index<usize> for Allocator<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get(index).expect("Use of node after free")
    }
}

impl<T> ::std::ops::IndexMut<usize> for Allocator<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.get_mut(index).expect("Use of node after free")
    }
}
