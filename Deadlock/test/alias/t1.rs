fn main() {
    let mut x = 10;     // x拥有所有权
    let y = &x;         // y是x的引用
    let z = y;          // z也是x的引用

    // x = 20;          // 错误: 不允许修改不可变引用
    // *y = 30;        // 错误: 不允许通过不可变引用修改x

    // Check the values by adding assertions or other checks
}


fn tt() {
    let mut a = 5;     // a拥有所有权
    let mut b = 10;    // b拥有所有权

    let c = &a;        // c是a的引用
    a = b;             // a现在拥有b的所有权

    // Check the values by adding assertions or other checks
}
