pub enum Instruction {
    Create { owner: String },
    Deposit { owner: String, amount: u64 },
    Transfer { from: String, to: String, amount: u64 },
    Balance { owner: String },
}