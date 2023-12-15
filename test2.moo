
fn print(x: str): void {
    ___write stdout x;
}

fn main(a: int, b: int): int {

    let MOO  = 10;
    let foo = 11;

    print((5 + 2 - foo) * 3 % MOO);
    ___write stderr 42;

    0
}
