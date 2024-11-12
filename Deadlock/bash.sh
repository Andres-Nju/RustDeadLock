cargo install --path .
cd test/double_lock_0
call-deadlock
cd ../..