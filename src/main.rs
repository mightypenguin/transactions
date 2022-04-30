use std::{env, fmt::Debug};
use std::str::FromStr;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use csv;
use std::process;

use serde::{Serialize, Deserialize, Serializer, Deserializer};

// Probably should use Arbitrary precision math library like "rug" instead of f64

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Transaction {
    #[serde(deserialize_with = "deserialize_type")]
    r#type: String,
    client: u16,
    tx: u32,
    #[serde(deserialize_with = "deserialize_amount")]
    amount: f64,
}

fn serialize_f64_to_4_decimals<S>(amount:&f64, s:S) -> Result<S::Ok, S::Error> where S: Serializer
{
    s.serialize_str(format!("{:.1$}",*amount,4).as_str())
}

// Handle empty/nonexistent amount field
fn deserialize_type<'de, D>(deserializer: D) -> Result<String, D::Error>
where D: Deserializer<'de> {
    let buf = String::deserialize(deserializer)?;
    let str = buf.to_lowercase(); //force lowercase

    return Ok(str);
}

// Handle empty/nonexistent amount field
fn deserialize_amount<'de, D>(deserializer: D) -> Result<f64, D::Error>
where D: Deserializer<'de> {
    let buf = String::deserialize(deserializer)?;
    let conversion = if !buf.is_empty() { 
        f64::from_str(&buf) 
    } else { 
        Ok(0.0) 
    };

    let res2 = if !conversion.is_err() { 
        Ok(conversion.unwrap()) //safe todo, we are checkint of is_error()
    } else { 
        Err(serde::de::Error::custom(format!("Invalid f64 string value: {}",buf))) 
    };

    return res2;
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
struct Balances {
    client: u16,
    #[serde(serialize_with = "serialize_f64_to_4_decimals")]
    available: f64,
    #[serde(serialize_with = "serialize_f64_to_4_decimals")]
    held: f64,
    #[serde(serialize_with = "serialize_f64_to_4_decimals")]
    total: f64,
    locked: bool
}

const TXN_TYPE_DEPOSIT:&str     = "deposit";
const TXN_TYPE_WITHDRAWAL:&str  = "withdrawal";
const TXN_TYPE_DISPUTE:&str     = "dispute";
const TXN_TYPE_RESOLVE:&str     = "resolve";
const TXN_TYPE_CHARGEBACK:&str  = "chargeback";

//const TXN_DISPUTABLE_TYPES:&'static [&'static str] = &[TXN_TYPE_DEPOSIT];

#[derive(Serialize, Deserialize, Debug)]
struct Account {
    balances:Balances,
    transactions:Vec<Transaction>
}

impl Account {
    pub fn find_transaction(&self, find_txn:&Transaction) -> std::option::Option<&Transaction> {
        let txn = self.transactions.iter()
                    .find(|txn| txn.tx==find_txn.tx );
        //We really should restrict to deposits...  && TXN_DISPUTABLE_TYPES.contains(&txn.r#type.as_str())

        return txn;
    }
}

fn process_transaction_file(file_name:&String, accounts:&mut HashMap<u16,Account>) -> Result<(), Box<dyn Error>> {
    let file_handle = File::open(file_name).unwrap(); //program should exit with error if file doesn't exist
    let file_reader = BufReader::new(file_handle);
    let mut rdr = csv::ReaderBuilder::new().trim(csv::Trim::All).from_reader(file_reader);
    
    for result in rdr.deserialize() {
        let txn: Transaction = result?;
        //println!("{:?}", txn);

        if !accounts.contains_key(&txn.client) {
            accounts.insert(txn.client, Account { 
                balances: Balances { client:txn.client, available:0.0, held:0.0, total:0.0, locked: false },
                transactions: Vec::new() 
            });
        }
        let mut account = accounts.get_mut(&txn.client).unwrap(); //if check/insert failed, then we should probably panic

        match txn.r#type.as_str() {
            TXN_TYPE_DEPOSIT => {
                //check locked?
                if txn.amount > 0.0 && (*account).balances.locked == false {
                    account.balances.available += txn.amount;
                    account.balances.total += txn.amount;

                    account.transactions.push(txn);
                }
                else
                {
                    //error transaction amount is negative
                }
                
            }
            TXN_TYPE_WITHDRAWAL => {
                //check locked?
                if txn.amount > 0.0 && account.balances.locked == false {
                    if txn.amount <= account.balances.available {
                        account.balances.available -= txn.amount;
                        account.balances.total -= txn.amount;

                        account.transactions.push(txn); 
                    }
                    else
                    {
                        //transaction amount is > available error
                    }
                }
                else
                {
                    //error transaction amount is negative
                }
            }
            TXN_TYPE_DISPUTE => {
                // Check if txn already disputed? Check if chargeback?
                let disputed_txn_res = account.find_transaction(&txn);
                if disputed_txn_res.is_none() {
                    //txn does not exist but that's ok, says the requirements
                    continue;
                }
                let disputed_txn = disputed_txn_res.unwrap().to_owned(); //protected by is_none() check

                // Check to prevent available going negative?
                if disputed_txn.amount > 0.0 {
                    if disputed_txn.amount <= account.balances.available {
                        account.balances.available -= disputed_txn.amount;
                        account.balances.held += disputed_txn.amount;

                        account.transactions.push(txn);
                    }
                    else
                    {
                        //transaction amount is > available error
                    }
                }
                else
                {
                    //error transaction amount is negative
                }
            }
            TXN_TYPE_RESOLVE => {
                // Check if txn already resolved or disputed?
                let resolved_txn_res = account.find_transaction(&txn);
                if resolved_txn_res.is_none() {
                    //txn does not exist but that's ok, says the requirements
                    continue;
                }
                let resolved_txn = resolved_txn_res.unwrap().to_owned(); //protected by is_none() check

                // Check to prevent held going negative?
                if resolved_txn.amount > 0.0 {
                    if resolved_txn.amount <= account.balances.held {
                        account.balances.held -= resolved_txn.amount;
                        account.balances.available += resolved_txn.amount;

                        account.transactions.push(txn);
                    }
                    else
                    {
                        //transaction amount is > available error
                    }
                }
                else
                {
                    //error transaction amount is negative
                }
            }
            TXN_TYPE_CHARGEBACK => {
                
                // Check if txn already chargeback, or if resolved, or if actually disputed?
                let chargeback_txn_res = account.find_transaction(&txn);
                if chargeback_txn_res.is_none() {
                    //txn does not exist but that's ok, says the requirements
                    continue;
                }
                let chargeback_txn = chargeback_txn_res.unwrap().to_owned(); //protected by is_none() check

                // Check to prevent held/total going negative?
                if chargeback_txn.amount > 0.0 {
                    account.balances.held -= chargeback_txn.amount;
                    account.balances.total -= chargeback_txn.amount;

                    account.balances.locked = true;

                    account.transactions.push(txn);
                }
                else
                {
                    //error transaction amount is negative
                }
            }
            _ => {
                //warn invalid transaction type
            }
        }

        //println!("Account:{:?}", *account);
    }

    Ok(())
}

fn write_output(accounts:&HashMap<u16,Account>) -> Result<(), Box<dyn Error>> {
    let mut wr = csv::Writer::from_writer(std::io::stdout());
    for (_, acct) in accounts.iter() { 
        wr.serialize(acct.balances).unwrap(); //Safe, and no error status to look at
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_name: &String = &args[1];

    let mut accounts:HashMap<u16,Account> = HashMap::new();

    if let Err(err) = process_transaction_file(file_name, &mut accounts) {
        println!("Error processing file: '{}', err:{}", file_name, err);
        process::exit(1);
    }

    write_output(&accounts).unwrap(); //Should probably handle error conditions

}
