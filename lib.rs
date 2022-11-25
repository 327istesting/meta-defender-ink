#![cfg_attr(not(feature = "std"), no_std)]



#[ink::contract]
mod meta_defender {

    use ink::prelude::{
        vec::Vec,
    };

    use ink::storage::Mapping;



    #[derive(scale::Encode, scale::Decode, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    #[derive(Default)]
    struct ProviderInfo {
        index: u128,
        participation_time: Timestamp,
        stoken_amount: u128,
        rdebt:u128,
        sdebt:u128,
    }

    #[derive(scale::Encode, scale::Decode, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    #[derive(Default)]
    struct PolicyInfo {
        id: u128,
        beneficiary: AccountId,
        coverage: u128,
        deposit: u128,
        start_time: Timestamp,
        effective_until: Timestamp,
        latest_provider_index: u128,
        delta_acc_sps: u128,
        is_claimed: bool,
        in_claim_applying: bool,
        is_canceled: bool,
    }


    #[derive(scale::Encode, scale::Decode, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    #[derive(Default)]
    struct HistoricalProviderInfo {
        index_before: u128, 
        stoken_amount_before: u128, 
        ftoken: u128,
        acc_sps_while_left: u128, 
        sdebt_before: u128, 
    }

    // The Meta_Defender result types.
    pub type Result<T> = core::result::Result<T, Error>;

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        NotJudger,
        NotOfficial,
        InsufficientCoverage,
        ExistingUnderWriter,
        NotUnderwriter,
    }


    #[ink(storage)]
    #[derive(Default)]
    pub struct MetaDefender {
        provider_map: Mapping<AccountId, ProviderInfo>,
        historical_provider_map: Mapping<AccountId, HistoricalProviderInfo>,
        user_policies: Mapping<AccountId, Vec<PolicyInfo>>,
        policies: Mapping<u128, PolicyInfo>,

        provider_count: u128, 
        exchange_rate: u128,
        policy_count: u128, 
        acc_rps: u128, 
        acc_sps: u128, 
        acc_sps_down: u128, 
        token_staked_here: u128,
        stoken_supply: u128, 
        token_frozen_here: u128,
        total_coverage: u128, 
        k_last: u128, 
        latest_unfrozen_index: u128, 
    
        initial_fee: u128,
        min_fee: u128, 
    
        judger: AccountId, 
        official: AccountId, 
    
        claimable_team_reward: u128,
        virtual_param: u128,
        provider_leaving: bool,
        historical_provider_leaving : bool,

        is_valid_mining_proxy: Mapping<AccountId, bool>,
    }

    impl MetaDefender {
        #[ink(constructor)]
        pub fn new(judger: AccountId, official: AccountId, virtual_param:u128) -> Self {
            let provider_map = Default::default();
            let historical_provider_map = Default::default();
            let user_policies = Default::default();
            let policies = Default::default();
            let is_valid_mining_proxy = Default::default();
            let initial_fee = 2000;

            MetaDefender { 
                provider_map, 
                historical_provider_map, 
                user_policies, 
                policies, 
                provider_count: 0, 
                policy_count : 0,
                exchange_rate: 100000, 
                acc_rps: 0, 
                acc_sps: 0, 
                acc_sps_down: 0, 
                token_staked_here: 0, 
                stoken_supply: 0, 
                token_frozen_here: 0, 
                total_coverage: 0, 
                k_last: 0, 
                latest_unfrozen_index: 0, 
                initial_fee, 
                min_fee: 2000, 
                judger, 
                official, 
                claimable_team_reward: 0, 
                virtual_param, 
                provider_leaving: false, 
                historical_provider_leaving: false, 
                is_valid_mining_proxy,
            }

        }


        /// This message changes the judger address.
        /// 
        /// Only current judger can call this message, if not, return NotJudger Error.
        #[ink(message)]
        pub fn judger_transfer(&mut self, judger: AccountId)  -> Result<()>{
            let caller = self.env().caller();
            if caller == self.judger{
                Err(Error::NotJudger)
            } else{
                self.judger  = judger;
                Ok(())
            }
        }


        /// This message changes the official address.
        /// 
        /// Only current official can call this message, if not, return NotOfficial Error.
        #[ink(message)]
        pub fn official_transfer(&mut self, official: AccountId)  -> Result<()>{
            let caller = self.env().caller();
            if caller == self.official{
                Err(Error::NotOfficial)
            } else{
                self.official  = official;
                Ok(())
            }
        }

        /// This message can add a mining proxy or terminate an existing mining proxy
        #[ink(message)]
        pub fn valid_mining_proxy_manage(&mut self, proxy: AccountId, _bool: bool)  -> Result<()>{
            let caller = self.env().caller();
            if caller == self.official{
                Err(Error::NotOfficial)
            } else{
                self.is_valid_mining_proxy.insert(proxy, &_bool);
                Ok(())
            }
        }


        /// This message can return current useable capital
        /// 
        /// If current total staked token is larger than total coverage, return 0
        #[ink(message)]
        pub fn get_useable_capital(&self)  -> u128 {
            if self.token_staked_here >= self.total_coverage{
                self.token_staked_here - self.total_coverage
            } else {
                0
            }

        }


        /// This message can calculate the current premium rate
        /// 
        /// If current useable capital is zero, return 0
        #[ink(message)]
        pub fn get_fee(&self)  -> u128 {
            let useable_capital = self.get_useable_capital();
            if useable_capital != 0 {
                let fee = self.k_last/(useable_capital + self.virtual_param);
                fee
            }else{
                0
            }
        }
    
        /// User buys a cover for himself with the specific coverage
        #[ink(message)]
        pub fn buy_cover(&mut self, coverage: u128)  -> Result<()> {
            
            let useable_capital = self.get_useable_capital();
            if useable_capital == 0 || coverage > useable_capital*2/100 {
                Err(Error::InsufficientCoverage)
            } else {
                let beneficiary = self.env().caller();
                let fee = self.get_fee();
                let cover_fee = coverage * fee / 100_000;
                let deposit = cover_fee * 5 / 100;
                let _total_pay = cover_fee + deposit;

                // aUSD.transferFrom(msg.sender, address(this), totalPay); //支付保费+押金

                self.total_coverage += coverage;
                let delta_acc_sps = coverage * 10_000_000_000_000 / self.stoken_supply;
                self.acc_sps += delta_acc_sps;


                // 5% goes to the team, remaining goes to underwriters
                let reward_for_team = cover_fee * 5 / 100;
                self.claimable_team_reward += reward_for_team;
                let reward_for_providers = cover_fee - reward_for_team;
                let delta_acc_rps = reward_for_providers * 10_000_000_000_000 / self.stoken_supply;
                self.acc_rps += delta_acc_rps;

                let start_time = self.env().block_timestamp();
                let effective_until = start_time + 90 * 86_400_000;

                
                let latest_provider_index = self.provider_count;
                
                let policy = PolicyInfo{
                    id: self.policy_count.clone(),
                    beneficiary,
                    coverage,
                    deposit,
                    start_time,
                    effective_until,
                    latest_provider_index,
                    delta_acc_sps,
                    is_claimed: false,
                    in_claim_applying: false,
                    is_canceled: false,
                };

                self.policies.insert(&self.policy_count, &policy);


                match self.user_policies.get(beneficiary) {
                    Some(mut v) => v.push(policy),
                    None => {
                        self.user_policies.insert(&beneficiary, &vec![policy]);
                    }
                }

                self.policy_count += 1;

                Ok(())
            }

        }

        
        #[ink(message)]
        pub fn provide_capital(&mut self, amount: u128)  -> Result<()> {
            let provider = self.env().caller();
            if let Some(_v) = self.provider_map.get(provider){
                return Err(Error::ExistingUnderWriter);
            }

            let index = self.provider_count.clone(); 
            let participation_time = self.env().block_timestamp();
            let stoken_amount = amount * 100_000 / self.exchange_rate;
            let rdebt = stoken_amount * self.acc_rps / 10_000_000_000_000;
            let sdebt = stoken_amount * self.acc_sps / 10_000_000_000_000;
            
            self.stoken_supply += stoken_amount;

            let provider_info = ProviderInfo{
                index,
                participation_time,
                stoken_amount,
                rdebt,
                sdebt,
            };

            self.provider_map.insert(provider, &provider_info);

            let pre_useable_capital = self.get_useable_capital().clone();
            self.token_staked_here += amount;
            let current_useable_capital = self.get_useable_capital();

            self.update_k_last_by_provider(pre_useable_capital, current_useable_capital); //更新kLast
            self.provider_count += 1;

            Ok(())

        }


        fn update_k_last_by_provider(&mut self, pre_useable_capital: u128, current_useable_capital: u128) {
            if self.provider_count == 0 {
                self.k_last = self.initial_fee * (current_useable_capital + self.virtual_param); 
            }else{
                let fee = self.k_last / (pre_useable_capital + self.virtual_param);
                self.k_last = fee * (current_useable_capital + self.virtual_param);
            }
        }


        pub fn get_reward(&self, address: AccountId) -> Result<()> {
            if 
            NotUnderwriter
            Ok(())
        }
    }
}
