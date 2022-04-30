# Transactions

## Testing
I used test input and output files with diffing to validate handling of various txn types.
I probably should have used unit tests but ran out of time.
The type system should cover all of this and I checked all usage of "unwrap" to verify safety/correctness and added notes in the code for each usage.

## Safety
Probably should have used an arbitrary precision math library instead of f64.
Extremely large transactions amounts may lose some precision.
It's just a matter of adding the serialize/deserialize handlers for those custom arbitrary math (e.g. rug library) types.

## Correctness
Forcing transaction amounts to be a (0.0) value (when not set in CSV input files) means additional memory usage, but simplifies coding since the program doesn't have to check for the existence of Optional fields.
I have stubs for error handling for the situations I could think of, and in other cases I have comments where I would normally manually verify requirements in more detail.

Transactions are tracked in the struct per user Account.
(Ideally this would be kept in a DB or microservice of some kind)

Dispute/Resolve/Chargeback logic only seems to make sense for deposits so that logic should be restricted to those?
I wrote this to follow the spec, which doesn't restrict which kinds of transactions those actions apply to.

Lots of safety checkes could have been implemented to prevent withdrawing more than available/total balances, respecting locked account status, handling negative transactions amounts etc.
But I followed the spec, in real life I would have confirmed the desired behavior with those writing the requirements.
