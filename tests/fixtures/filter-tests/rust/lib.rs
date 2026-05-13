pub fn production() {
    shared();
}

fn shared() {}

#[cfg(test)]
mod tests {
    fn hidden_test() {
        shared();
    }
}
