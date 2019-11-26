/*
  # Storage
  We do not store all state on a single HashMap, instead we store every Lottery seperately.
  This is so we can save gas costs, and avoid unnecessary serializations / deserializations.
*/
#![no_std]
#![allow(unused_attributes)]

extern crate eng_wasm;
extern crate eng_wasm_derive;
extern crate rustc_hex;
extern crate serde;
extern crate serde_derive;
extern crate std;

use eng_wasm::*;
use eng_wasm_derive::{pub_interface, eth_contract};
use rustc_hex::ToHex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::collections::HashMap;
//use std::assert_eq;
use std::f64;


// constant variables
static MAX_PARTICIPANTS: u16 = 1000;

// private state keys
static OWNERSHIP: &str = "OWNER";
static FLOATLIST: &str = "FLOATLIST";
static FLOATLISTSIZE: &str = "FLOATLISTSIZE";
static FIXLIST: &str = "FIXLIST";
static FIXLISTSIZE: &str = "FIXLISTSIZE";
static SWAPS: &str = "SWAPS";
static SWAP: &str = "SWAP_"; 
static ORDERLIST: &str = "ORDERLIST";
static DEFAULTLIST: &str = "DEFAULTLIST";// dynamically generated afterwards "LOTTERY_<ID>"

static PRICE: &str = "PRICE";

// owner
#[derive(Serialize, Deserialize)]
pub struct Ownership {
  owner_addr: String,
  //deposit_addr: H160,
}

// a hashset of whitelisted addresses
#[derive(Serialize, Deserialize)]
pub struct Floatlist(HashSet<String>);

#[derive(Serialize, Deserialize)]
pub struct Fixlist(HashSet<String>);

#[derive(Serialize, Deserialize)]
pub struct Defaultlist(HashSet<U256>);

// incremental lottery number, used to create new lotteries
#[derive(Serialize, Deserialize)]
pub struct Swaps(U256);

#[derive(Serialize, Deserialize)]
pub struct Price(U256);

// incremental lottery number, used to create new lotteries
#[derive(Serialize, Deserialize)]
pub struct Fixlistsize(U256);

// incremental lottery number, used to create new lotteries
#[derive(Serialize, Deserialize)]
pub struct Floatlistsize(U256);

// lottery status enum
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
enum SwapStatus {
  ORDERING = 0,
  CONTINUE = 1,
  COMPLETE = 2,
  DEFAULT = 3,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
enum FIX_or_FLOAT {
  FIX = 0,
  FLOAT = 1,
  None = 2,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    address: String,
    deposit: U256,
    position: FIX_or_FLOAT,
    exposure: U256,
    swaps: HashSet<U256>,
}
// lottery
#[derive(Serialize, Deserialize)]
pub struct Swap {
  id: U256,
  Owner: String,
  FIX_or_FLOAT: FIX_or_FLOAT,
  Amount: U256,
  participants: HashMap<String, U256>,
  Oppornent: String,
  Maturity: U256,
  PayRate: U256,
  ReceiveRate: U256,
  Status: SwapStatus,
}


// lottery info
type SwapInfo = (U256, U256, U256, U256, U256, U256);

//#[eth_contract("Deposit.json")]
struct EthContract;

pub struct Contract;

#[pub_interface]
pub trait ContractInterface {
  fn construct(owner_addr: H160) -> ();
  fn create_swap(
    fix_or_float: U256,
    amount: U256,
    maturity: U256,
    rate: U256,
    owner_raw: H160,
  ) -> ();
  fn get_swap_size() -> U256;
  fn join_swap(swap_num: U256, address_raw: H160, rate: U256) -> ();
  fn get_swap_info(swap_num: U256) -> SwapInfo;
  fn swapDecision(swap_num: U256, oppornent_raw: H160, swapowner_raw: H160) -> ();
  fn priceWash(swap_num: U256, new_price: U256) -> ();
  fn complete(swap_num: U256, now: U256) -> ();
  fn deposit(address_raw: H160, amount: U256) -> ();
}

// returns a Lottery state key, ex: "LOTTERY_1"
fn create_swap_key(swap_num: U256) -> String {
  let mut key = String::from(SWAP);
  key.push_str(&swap_num.to_string());

  return key;
}

// add prefix "0x" to address string
fn h160_to_string(address: H160) -> String {
  let addr_str: String = address.to_hex();

  return [String::from("0x"), addr_str].concat();
}

// secret fns
impl Contract {
  fn get_ownership() -> Ownership {
    match read_state!(OWNERSHIP) {
      Some(ownership) => ownership,
      None => panic!("ownership should already exist"),
    }
  }

  fn get_user(address: &String) -> User {
    match read_state!(address) {
      Some(user) => user,
      None => {
        User {
        address: address.to_string(),
        deposit: U256::from(0),
        position: FIX_or_FLOAT::None,
        exposure: U256::from(0),
        swaps: HashSet::new(),
      }
    }
    }
  }

  fn get_fixlist_size() -> U256 {
    match read_state!(FIXLISTSIZE) {
      Some(size) => size,
      None => U256::from(0),
    }
  }

  fn get_floatlist_size() -> U256 {
    match read_state!(FLOATLISTSIZE) {
      Some(size) => size,
      None => U256::from(0),
    }
  }

  fn get_fixlist() -> HashSet<String> {
    match read_state!(FIXLIST) {
      Some(fixlist) => fixlist,
      None => HashSet::new(),
    }
  }

  fn get_floatlist() -> HashSet<String> {
    match read_state!(FLOATLIST) {
      Some(floatlist) => floatlist,
      None => HashSet::new(),
    }
  }

  fn get_defaultlist() -> HashSet<U256> {
    match read_state!(DEFAULTLIST) {
      Some(defaultlist) => defaultlist,
      None => HashSet::new(),
    }
  }

  fn get_swaps() -> U256 {
    match read_state!(SWAPS) {
      Some(swaps) => swaps,
      None => U256::from(0),
    }
  }

  fn get_price() -> U256 {
    match read_state!(PRICE) {
      Some(price) => price,
      None => U256::from(0),
    }
  }

  fn get_swap(swap: &str) -> Swap {
    match read_state!(swap) {
      Some(swap) => swap,
      None => panic!("swap does not exist"),
    }
  }

  fn default(address: String) -> (){
    //let address: String = &h160_to_string(address_raw); 
    let mut user = Self::get_user(&address);
    let swaps = user.swaps;
    let mut defaultlist = Self::get_defaultlist();
    for swap_num in swaps{
      let swap_key = &create_swap_key(swap_num);
      let mut swap = Self::get_swap(swap_key);
      swap.Status = SwapStatus::DEFAULT;
      defaultlist.insert(swap_num);
      write_state!(swap_key => swap);
    }
    write_state!(DEFAULTLIST => defaultlist);
    user.swaps = HashSet::new();
    user.exposure = U256::from(0);
    user.deposit = U256::from(0);
    write_state!(&address => user);
  }

  fn changeExposure(address: String, amount: U256, fix_or_float: U256, swap_num: U256) -> (){
    //let address: str = &h160_to_string(address_raw);
    let mut user = Self::get_user(&address);
    if(U256::from(user.position as usize) == fix_or_float){
      user.exposure += amount;
      user.swaps.insert(swap_num);
      write_state!(&address => user);
    }else if(user.position == FIX_or_FLOAT::FIX){
      if(user.exposure >= amount){
        user.exposure -= amount;
        user.swaps.insert(swap_num);
        write_state!(&address => user);
      }else{

        user.position = FIX_or_FLOAT::FLOAT;
        user.exposure = amount - user.exposure;
        user.swaps.insert(swap_num);
        
       let mut floatlist = Self::get_floatlist();
       floatlist.insert(address.to_string());
       user.swaps.insert(swap_num);
       let mut fixlist = Self::get_fixlist();
       fixlist.remove(&address);
       user.swaps.insert(swap_num);
       write_state!(FLOATLIST => floatlist);
       write_state!(FIXLIST => fixlist);
       write_state!(&address => user);
      }
    }else if(user.position == FIX_or_FLOAT::FLOAT){
        if(user.exposure >= amount){
          user.exposure -= amount;
          write_state!(&address => user);
        }else{
          user.position = FIX_or_FLOAT::FIX;
          user.exposure = amount - user.exposure;

          let mut fixlist = Self::get_fixlist();
          fixlist.insert(address.to_string());
          let mut floatlist = Self::get_floatlist();
          floatlist.remove(&address);
          write_state!(&address => user);
          write_state!(FIXLIST => fixlist);
          write_state!(FLOATLIST => floatlist);
      }
    }else if (user.position == FIX_or_FLOAT::None){
      if(fix_or_float == U256::from(FIX_or_FLOAT::FIX as usize)){
       let mut fixlist = Self::get_fixlist();
       fixlist.insert(address);
       write_state!(FIXLIST => fixlist);
      }
    }
  }

 fn changeExposureClose(address: String, amount: U256, fix_or_float: usize, swap_num: U256) -> (){
    let mut user = Self::get_user(&address);
    if(user.position as usize != fix_or_float){
      user.exposure += amount;
      user.swaps.remove(&swap_num);
      write_state!(&address => user);
    }else if(user.position == FIX_or_FLOAT::FIX){
      if(user.exposure >= amount){
        user.exposure -= amount;
        user.swaps.remove(&swap_num);
        write_state!(&address => user);
      }else{
        user.position = FIX_or_FLOAT::FLOAT;
        user.exposure = amount - user.exposure;
        user.swaps.remove(&swap_num);
        write_state!(&address => user);
      }
      }else if(user.position == FIX_or_FLOAT::FLOAT){
        if(user.exposure >= amount){
          user.exposure -= amount;
          user.swaps.remove(&swap_num);
          write_state!(&address => user);
        }else{
          user.position = FIX_or_FLOAT::FIX;
          user.exposure = amount - user.exposure;
          user.swaps.remove(&swap_num);
          write_state!(&address => user);
      }
    }
  }
  // TODO: needs security check, see: https://forum.enigma.co/t/enigmasimulation/1070/16?u=nioni
  // thus not used
  // fn is_whitelisted(address: H160) -> bool {
  //   let mut whitelist = Self::get_whitelist();

  //   return whitelist.contains(&address);
  // }
}

// public fns
impl ContractInterface for Contract {
  #[no_mangle]
  fn construct(owner_addr: H160) -> () {
    write_state!(OWNERSHIP => Ownership {
      owner_addr: h160_to_string(owner_addr),
      //contract_addr: contract_addr, 
    });
    write_state!(PRICE => U256::from(0));
  }

  #[no_mangle]
  fn create_swap (
    fix_or_float: U256,
    amount: U256,
    maturity: U256,
    rate: U256,
    owner_raw: H160,
  ) -> () {
    let owner = h160_to_string(owner_raw); 
    let swaps = Self::get_swaps();
    let mut FF: FIX_or_FLOAT = FIX_or_FLOAT::FLOAT;
    if (fix_or_float == U256::from(FIX_or_FLOAT::FIX as usize)){FF = FIX_or_FLOAT::FIX}

    // make new id
    let id = swaps.checked_add(U256::from(1)).unwrap();
     //todo: Verify Account
    let swap = Swap {
      id: id,
      FIX_or_FLOAT: FF,
      Amount: amount,
      participants: HashMap::new(),
      Oppornent: owner.to_string(),
      Maturity: maturity,
      Owner: owner.to_string(),
      PayRate: rate,
      ReceiveRate: U256::from(0),
      Status: SwapStatus::ORDERING,
    };

    write_state!(SWAPS => swaps, &create_swap_key(swaps) => swap);
  }

  #[no_mangle]
  fn get_swap_size() -> U256 {
    return Self::get_swaps();
  }

  #[no_mangle]
  fn join_swap(swap_num: U256, address_raw: H160, rate: U256) -> () {
    let address = h160_to_string(address_raw); 
    let ownership = Self::get_ownership();
    let swap_key = &create_swap_key(swap_num);
    let mut swap = Self::get_swap(swap_key);

    // check if max amount of lottery participants reached
    assert!(
      swap.Status == SwapStatus::ORDERING,
      "already started."
    );

    // check if participant exists
    assert!(
      !swap.participants.contains_key(&address),
      "participant already exists"
    );

    // insert new participants
    swap.participants.insert(address, rate);

    write_state!(swap_key => swap);
  }

  #[no_mangle]
  fn get_swap_info(swap_num: U256) -> SwapInfo {
    let swap_key = &create_swap_key(swap_num);
    let swap = Self::get_swap(swap_key);

    return (
      swap.id,
      U256::from(swap.FIX_or_FLOAT as usize),
      U256::from(swap.Status as usize),
      swap.Maturity,
      swap.PayRate,
      swap.ReceiveRate,
    );
  }

   #[no_mangle]
  fn swapDecision(swap_num: U256, oppornent_raw: H160, swapowner_raw: H160) -> (){
    let swapowner = h160_to_string(swapowner_raw); 
    let oppornent = h160_to_string(oppornent_raw); 
    let swap_key = &create_swap_key(swap_num);
    let mut swap = Self::get_swap(swap_key);
    assert!(swap.Owner == swapowner,"you do't have this swap." );

    let participants = swap.participants;
    match participants.get(&oppornent) {
      Some(rate) => {
        let swap_key = &create_swap_key(swap_num);
        swap.Oppornent = oppornent;
        swap.participants = HashMap::new();
        swap.Status = SwapStatus::COMPLETE;
        swap.ReceiveRate = *rate;
        Self::changeExposure(swapowner, swap.Amount, U256::from(swap.FIX_or_FLOAT as usize), swap_num);
        write_state!(swap_key => swap);
      },
      None => panic!("oppornent does not exist"),
    }
  }

  #[no_mangle]
  fn priceWash(swap_num: U256, new_price: U256) -> (){
    let price: U256 = Self::get_price();
    if(price > new_price) {
      let price_dif: U256 = price.checked_sub(new_price).unwrap();
      let floatlist = Self::get_floatlist();
      for useraddress in floatlist{ 
        let mut user = Self::get_user(&useraddress);
        if (user.deposit > user.exposure.checked_mul(price_dif).unwrap()){
          user.deposit = user.deposit.checked_sub(user.exposure.checked_mul(price_dif).unwrap()).unwrap();
          write_state!(&useraddress => user);
        }else{
          Self::default(useraddress);
        }
      }
    let fixlist = Self::get_fixlist();
    for useraddress in fixlist{
      let mut user = Self::get_user(&useraddress);
      user.deposit = user.deposit.checked_add(user.exposure.checked_mul(price_dif).unwrap()).unwrap();
      write_state!(&useraddress => user);
      }
    }else if(price < new_price) {
      let price_dif: U256 = new_price.checked_sub(price).unwrap();
      let fixlist = Self::get_fixlist();
      for useraddress in fixlist{
        let mut user = Self::get_user(&useraddress);
        if (user.deposit > user.exposure.checked_mul(price_dif).unwrap()){
          user.deposit = user.deposit.checked_sub(user.exposure.checked_mul(price_dif).unwrap()).unwrap();
          write_state!(&useraddress => user);
        }else{
          Self::default(useraddress);
        }
      }
      let floatlist = Self::get_floatlist();
      for useraddress in floatlist{
        let mut user = Self::get_user(&useraddress);
        user.deposit = user.deposit.checked_add(user.exposure.checked_mul(price_dif).unwrap()).unwrap();
        write_state!(&useraddress => user);
      }
    }
  }
 
  #[no_mangle]
  fn complete(swap_num: U256, now: U256) -> () {
      // todo: get current time(now won't be needed)
      let swap_key = &create_swap_key(swap_num);
      let swap = Self::get_swap(swap_key);
      assert!(swap.Maturity < now, "maturity hasn't come yet");
      swap.Status == SwapStatus::COMPLETE;
      Self::changeExposureClose(swap.Owner.to_string(), swap.Amount, swap.FIX_or_FLOAT as usize, swap_num);
      write_state!(swap_key => swap);
  }

  #[no_mangle]
  fn deposit(address_raw: H160, amount: U256) -> () {
    
    // todo: this function is not needed if ETH => ENG solution is found.
    let address = &h160_to_string(address_raw); 
    let mut user = Self::get_user(address);
    user.deposit += amount;
    write_state!(address => user);
  }
}

