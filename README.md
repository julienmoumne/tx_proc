# automated testing

[tests/lib_test.rs](tests/lib_test.rs) : functional/business test cases 

[tests/main_test.rs](tests/main_test.rs) : end to end and CSV decode/encode, using the CSV files located in [tests/data](tests/data)

on top of the documented edge cases found in the instructions, the following unspecified cases are tested :

## unspecified edge cases

- negative and overflowing integers for user id and transaction id

record is considered invalid and is skipped

- negative amounts in Deposit or Withdrawal `withdrawal, 1, 4, -1.5`

record is considered invalid and is skipped

- no amount in Deposit or Withdrawal `withdrawal, 1, 4`,  `withdrawal, 1, 4, `

record is considered invalid and is skipped

- repeated transactions

repeated transactions are considered invalid and skipped

- Dispute/Resolve/Chargeback on a Withdrawal

record is considered invalid and is skipped

- Dispute/Resolve/Chargeback with the wrong client id specified

record is considered invalid and is skipped

- valid `Dispute->Resolve`, valid `Dispute->Resolve`, .., on same deposit

allowed

an alternative would be to only allow one Dispute->Resolve per deposit

- an amount is specified on a Dispute/Resolve/Chargeback `dispute, 1, 1, 222`

record is considered valid and the extraneous amount is discarded

an alternative would be to consider the record invalid and skip it

- transactions on locked account

record is considered invalid and is skipped

- UTF-8 encoding everywhere

command arguments and CSV input files must be encoded using UTF-8 otherwise the program may fail

- CSV with no headers

not processed

an alternative would be to allow CSV files with no headers


# efficiency

## big CSV files

the csv crate uses a `BufReader` of size `8 * (1 << 10) bytes = 8 KiB`

which means big CSV files will not saturate memory

## data structures

constant time reads are required for account and transactions

using `Vec` would risk saturating the memory even with one record (transaction id = 4294967295)

HashMaps are used, with amortised O(1) reads

## transactions

for several use cases, such as looking up the amount of a disputed deposit, it is required to have access to past processed transactions

it is possible to saturate the memory if too many transactions are processed

using an external service such as a database might be a solution

## What if these CSVs came from thousands of concurrent TCP streams?

### data consistency

as specified in the instructions, transactions IDs can not be ordered

transactions have to be executed in the order provided in the CSV input file

if multiple CSVs were processed concurrently, we would run the risk of inconsistent data

e.g. a dispute processed before an earlier un-processed deposit

we would need a way for the CSV input files to be sharded and linearized, meaning:
- no two input files be concurrently processed if they contain transactions for the same account
- transactions are ordered per input file but input files are also ordered

concepts such as event sourcing, queues, topic partitions comes to mind for such requirements

### shared memory

the current implementation relies on mutable data structures

sharing those data structures between threads is possible using locking with Arc/Mutex, and may be acceptable

until thread contention happens (too many tasks, IO operations while holding the lock, expensive computation while holding the lock)

then either concurrency optimized data structures such as concurrent hashmaps may be used 

or have only one owner of the data and use channels/message passing

or use an external service such as a database


# code guidelines

`cargo format` was used for formatting

`cargo clippy` was used to spot issues, all warnings have been fixed

# crates

on top of `serde` and `csv`, `rust_decimal` is used to conveniently and precisely manipulate amounts

# todo

benchmarks using criterion?