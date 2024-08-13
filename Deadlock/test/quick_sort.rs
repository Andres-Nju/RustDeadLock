fn quicksort(arr: &mut [i32]) {
    if arr.len() <= 1 {
        return;
    }

    let pivot_index = partition(arr);
    let (left, right) = arr.split_at_mut(pivot_index);
    
    quicksort(left);
    quicksort(&mut right[1..]); // 排除 pivot 元素
}

fn partition(arr: &mut [i32]) -> usize {
    let len = arr.len();
    let pivot_index = len - 1;
    let pivot = arr[pivot_index];

    let mut i = 0;
    for j in 0..pivot_index {
        if arr[j] <= pivot {
            arr.swap(i, j);
            i += 1;
        }
    }
    arr.swap(i, pivot_index);
    i
}

fn main() {
    let mut array = [3, 6, 8, 10, 1, 2, 1];
    println!("Original array: {:?}", array);

    quicksort(&mut array);
    println!("Sorted array: {:?}", array);
}
