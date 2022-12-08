#![cfg_attr(not(feature = "std"), no_std)]



#[ink::contract]
mod meta_defender {

    use ink::prelude::vec::Vec;
    use ink::storage::Mapping;
   
    use erc20::{Erc20, Erc20Ref , Erc20Error};
    use ink::env::call::FromAccountId;



    #[derive(scale::Encode, scale::Decode, Debug)]
    #[cfg_attr(
        feature = "std",
        derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout)
    )]
    #[derive(Default)]
    struct ProviderInfo {
        index: u128,
        participation_time: Timestamp,
        stoken_amount: Balance,
        rdebt:Balance,
        sdebt:Balance,
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
        coverage: Balance,
        deposit: Balance,
        start_time: Timestamp,
        effective_until: Timestamp,
        latest_provider_index: u128,
        delta_acc_sps: Balance,
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
        stoken_amount_before: Balance, 
        ftoken: Balance,
        acc_sps_while_left: Balance, 
        sdebt_before: Balance, 
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
        NotValidUnderwriter,
        ProviderLeavingInProgress,
        NotHistoricalUnderwriter,
        HistoricalProviderLeavingInProgress,
        InsufficientSToken,
        NotExistedPolicy,
        AlreadyCancelledPolicy,
        NotExpiredPolicy,
        ClaimingInProgress,
        OnlyPolicyHolderCanCancel,
        PreviousPolicyNotCancelled,
        NotBeneficiary,
        AlreadyClaimedPolicy,
        InClaimingProgress,
        NotEffectivePolicy,
        NotInClaimingProgress,
        NotValidMiningProxy,
        InsufficientBalance,
        InsufficientAllowance,
        TransferError,
    }


    #[ink(storage)]
    pub struct MetaDefender {
        provider_map: Mapping<AccountId, ProviderInfo>,
        historical_provider_map: Mapping<AccountId, HistoricalProviderInfo>,
        user_policies: Mapping<AccountId, Vec<PolicyInfo>>,
        policies: Mapping<u128, PolicyInfo>,

        provider_count: u128, 
        exchange_rate: Balance,
        policy_count: u128, 
        acc_rps: Balance, 
        acc_sps: Balance, 
        acc_sps_down: Balance, 
        token_staked_here: Balance,
        stoken_supply: Balance, 
        token_frozen_here: Balance,
        total_coverage: Balance, 
        k_last: u128, 
        latest_unfrozen_index: u128, 
    
        initial_fee: u128,
        min_fee: u128, 
    
        judger: AccountId, 
        official: AccountId, 
    
        claimable_team_reward: Balance,
        virtual_param: Balance,
        provider_leaving: bool,
        historical_provider_leaving : bool,

        is_valid_mining_proxy: Mapping<AccountId, bool>,
        erc20: Erc20Ref,
        risk_reserve: AccountId,
    }


    impl MetaDefender {


        #[ink(constructor)]
        pub fn new(
            official: AccountId, 
            judger: AccountId,  
            risk_reserve: AccountId,
            virtual_param:Balance,
            erc20: AccountId) -> Self {
            let provider_map = Default::default();
            let historical_provider_map = Default::default();
            let user_policies = Default::default();
            let policies = Default::default();
            let is_valid_mining_proxy = Default::default();
            let initial_fee = 2000;
            let erc20 = Erc20Ref::from_account_id(erc20);
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
                erc20,
                risk_reserve,
            }

        }


        /// This message changes the judger address.
        /// 
        /// Only current judger can call this message, if not, return NotJudger Error.
        #[ink(message)]
        pub fn judger_transfer(&mut self, judger: AccountId)  -> Result<()>{
            let caller = self.env().caller();
            if caller != self.judger{
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
            if caller != self.official{
                Err(Error::NotOfficial)
            } else{
                self.official  = official;
                Ok(())
            }
        }

        #[ink(message)]
        pub fn team_claim(&mut self) -> Result<()>{
            let caller = self.env().caller();
            if caller == self.official{
                match self.erc20.transfer(self.official, self.claimable_team_reward){
                    Err(e) if e == Erc20Error::InsufficientAllowance => return Err(Error::InsufficientAllowance),
                    Err(e) if e == Erc20Error::InsufficientBalance => return Err(Error::InsufficientBalance),
                    Err(_e) => return Err(Error::TransferError),
                    Ok(_) => return Ok(()),
                }
            }else{
                return Err(Error::NotOfficial);
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
        pub fn get_useable_capital(&self)  -> Balance {
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
        pub fn get_fee(&self)  -> Balance {
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
        pub fn buy_cover(&mut self, coverage: Balance)  -> Result<()> {
            
            let useable_capital = self.get_useable_capital();
            if useable_capital == 0 || coverage > useable_capital*2/100 {
                Err(Error::InsufficientCoverage)
            } else {
                let beneficiary = self.env().caller();
                let fee = self.get_fee();
                let cover_fee = coverage * fee / 100_000;
                let deposit = cover_fee * 5 / 100;
                let total_pay = cover_fee + deposit;
                let this = self.env().account_id();

                match self.erc20.transfer_from(beneficiary, this, total_pay){
                    Err(e) if e == Erc20Error::InsufficientAllowance => return Err(Error::InsufficientAllowance),
                    Err(e) if e == Erc20Error::InsufficientBalance => return Err(Error::InsufficientBalance),
                    Err(_e) => return Err(Error::TransferError),
                    Ok(_) => return {
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
                                self.user_policies.insert(&beneficiary, &Vec::from([policy]));
                            }
                        }
        
                        self.policy_count += 1;
        
                        Ok(())
                    }
                    }
                }



        }

        
        #[ink(message)]
        pub fn provide_capital(&mut self, amount: Balance)  -> Result<()> {
            let provider = self.env().caller();
            let this = self.env().account_id();
            if let Some(_v) = self.provider_map.get(provider){
                return Err(Error::ExistingUnderWriter);
            }

            match self.erc20.transfer_from(provider, this , amount) {
                Err(e) if e == Erc20Error::InsufficientAllowance => return Err(Error::InsufficientAllowance),
                Err(e) if e == Erc20Error::InsufficientBalance => return Err(Error::InsufficientBalance),
                Err(_e) => return Err(Error::TransferError),
                Ok(_) => {
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
            }


        }


        fn update_k_last_by_provider(&mut self, pre_useable_capital: Balance, current_useable_capital: Balance) {
            if self.provider_count == 0 {
                self.k_last = self.initial_fee * (current_useable_capital + self.virtual_param); 
            }else{
                let fee = self.k_last / (pre_useable_capital + self.virtual_param);
                self.k_last = fee * (current_useable_capital + self.virtual_param);
            }
        }


        fn get_reward(&self, address: &AccountId) -> Balance {
            match self.provider_map.get(address){
                None =>  0,
                Some(v) => {
                    if v.stoken_amount != 0 {
                        v.stoken_amount * self.acc_rps / 10_000_000_000_000 - v.rdebt
                    }else{
                        0
                    }
                }
            }
        }

        #[ink(message)]
        pub fn provider_take_reward(&mut self) -> Result<()>{
            let caller = self.env().caller();
            match self.provider_map.get(caller) {
                None => Err(Error::NotUnderwriter),
                Some(v) if v.stoken_amount == 0 => Err(Error::NotValidUnderwriter),
                Some(mut v) => {
                    let reward = self.get_reward(&caller);
                    v.rdebt = v.stoken_amount * self.acc_rps / 10_000_000_000_000;
                    match self.erc20.transfer(caller, reward){
                        Err(e) if e == Erc20Error::InsufficientAllowance => return Err(Error::InsufficientAllowance),
                        Err(e) if e == Erc20Error::InsufficientBalance => return Err(Error::InsufficientBalance),
                        Err(_e) => return Err(Error::TransferError),
                        Ok(_) => return Ok(())
                    }
                }
            }
        }

        fn get_shadow(&self, provider: &ProviderInfo) -> Balance{
            if provider.index > self.latest_unfrozen_index{
                provider.stoken_amount * self.acc_sps / 10_000_000_000_000 - provider.sdebt
            }else{
                let delta = self.acc_sps - self.acc_sps_down;
                provider.stoken_amount * delta / 10_000_000_000_000
            }

        }

        fn get_shadow_historical_provider(&self, historical_provider: &HistoricalProviderInfo) -> Balance {
            if historical_provider.index_before > self.latest_unfrozen_index{
                historical_provider.stoken_amount_before * historical_provider.acc_sps_while_left / 10_000_000_000_000 - historical_provider.sdebt_before
            }else{
                if historical_provider.acc_sps_while_left >= self.acc_sps_down {
                    historical_provider.acc_sps_while_left - self.acc_sps_down
                } else{
                    let delta = historical_provider.acc_sps_while_left - self.acc_sps_down;
                    historical_provider.stoken_amount_before * delta / 10_000_000_000_000
                }
            }
        }


        fn register_historical_provider(&mut self, provider: &ProviderInfo, token_remain:Balance, withdrawable_capital:Balance, address: &AccountId){
            let index_before = provider.index;
            let stoken_amount_before = provider.stoken_amount;
            let token_left = token_remain - withdrawable_capital;
            let ftoken = token_left * 100_000 / self.exchange_rate;
            let acc_sps_while_left = self.acc_sps;
            let sdebt_before = provider.sdebt;
            let historical_provider = HistoricalProviderInfo{
                index_before, 
                stoken_amount_before,
                ftoken,
                acc_sps_while_left, 
                sdebt_before, 
            };
            self.token_frozen_here += token_left;

            self.historical_provider_map.insert(address, &historical_provider);
        }

        #[ink(message)]
        pub fn provider_abolish(&mut self) -> Result<()> {
            let caller = self.env().caller();
            match self.provider_map.get(caller) {
                None => Err(Error::NotUnderwriter),
                Some(_v) if self.provider_leaving == true => Err(Error::ProviderLeavingInProgress),
                Some(v) if v.stoken_amount == 0 => Err(Error::NotValidUnderwriter),
                Some(mut v) => {
                    self.provider_leaving = true;

                    let token_remain = v.stoken_amount * self.exchange_rate / 100_000;
                    let shadow = self.get_shadow(&v);

                    let withdrawable_capital: u128;
                    if token_remain >= shadow {
                        withdrawable_capital = token_remain - shadow;
                    } else{
                        withdrawable_capital = 0;
                    }

                    let reward = v.stoken_amount * self.acc_rps / 10_000_000_000_000 - v.rdebt;

                    self.register_historical_provider(&v, token_remain, withdrawable_capital, &caller);

                    self.stoken_supply -= v.stoken_amount;
                    v.stoken_amount = 0;
                    v.rdebt = 0;

                    let pre_useable_capital = self.get_useable_capital().clone();
                    self.token_staked_here -= token_remain;
                    let current_useable_capital = self.get_useable_capital();
                    self.update_k_last_by_provider(pre_useable_capital, current_useable_capital);

                    
                    if withdrawable_capital + reward > 0 {
                        match self.erc20.transfer(caller, withdrawable_capital + reward) {
                            Err(e) if e == Erc20Error::InsufficientAllowance => {
                                self.provider_leaving = false;
                                return Err(Error::InsufficientAllowance);
                            },
                            Err(e) if e == Erc20Error::InsufficientBalance => {
                                self.provider_leaving = false;
                                return Err(Error::InsufficientBalance)
                            },
                            Err(_e) => {
                                self.provider_leaving = false;
                                return Err(Error::TransferError)},
                            Ok(_) => {
                                self.provider_leaving = false;
                                return Ok(())
                            }
                        }
                    }
                self.provider_leaving = false;
                Ok(())
                }
            }
        }

        #[ink(message)]
        pub fn get_unfrozen_capital(&self) -> u128 {
            let caller = self.env().caller();
            if let Some(v) = self.historical_provider_map.get(caller) {
                let shadow = self.get_shadow_historical_provider(&v);
                if v.ftoken * self.exchange_rate / 100_000 <= shadow {
                    return 0;
                }else{
                    return v.ftoken * self.exchange_rate / 100_000 - shadow;
                }
            };

            match self.provider_map.get(caller) {
                None => return 0,
                Some(v) => {
                    let token_remain = v.stoken_amount * self.exchange_rate / 100_000;
                    if v.index > self.latest_unfrozen_index {
                        let shadow = v.stoken_amount * self.acc_sps / 10_000_000_000_000 - v.sdebt;
                        if token_remain >= shadow {
                            return token_remain - shadow;
                        }else{
                            return 0;
                        }
                    }else{
                        let delta = self.acc_sps - self.acc_sps_down;
                        let shadow = v.stoken_amount * delta / 10_000_000_000_000;
                        if token_remain >= shadow {
                            return token_remain - shadow;
                        }else{
                            return 0;
                        }
                    }
                }
            }
        }

        #[ink(message)]
        pub fn historical_provider_withdraw(&mut self) -> Result<()>{
            let caller = self.env().caller();
            match self.historical_provider_map.get(&caller){
                None => return Err(Error::NotHistoricalUnderwriter),
                Some(_v) if self.historical_provider_leaving == true => return Err(Error::HistoricalProviderLeavingInProgress),
                Some(mut v) => {
                    self.historical_provider_leaving = true;

                    let shadow = self.get_shadow_historical_provider(&v);
                    
                    if v.ftoken * self.exchange_rate / 100_000 <= shadow {
                        return Err(Error::InsufficientSToken);
                    }else {
                        match self.erc20.transfer(caller, v.ftoken*self.exchange_rate/100_000 - shadow){
                            Err(e) if e == Erc20Error::InsufficientAllowance => {
                                self.historical_provider_leaving = false;
                                return Err(Error::InsufficientAllowance)
                            },
                            Err(e) if e == Erc20Error::InsufficientBalance => {
                                self.historical_provider_leaving = false;
                                return Err(Error::InsufficientBalance)
                            },
                            Err(_e) => {
                                self.historical_provider_leaving = false;
                                return Err(Error::TransferError)
                            },
                            Ok(_) => {
                                self.token_frozen_here -= v.ftoken * self.exchange_rate / 100_000 - shadow;
                                v.ftoken = shadow * 100_000 / self.exchange_rate;
                                self.historical_provider_leaving = false;
                                return Ok(());}
                        }    
                        
                    }
                }
            }
        }

        #[ink(message)]
        pub fn try_policy_cancel(&mut self, id: u128) -> Result<()> {
            match self.policies.get(id) {
                None => return Err(Error::NotExistedPolicy),
                Some(v) if v.is_canceled == true =>  return Err(Error::AlreadyCancelledPolicy),
                Some(mut v) => {
                    if id == 0 {
                        match self.execute_cancel(&mut v){
                            Err(e) => return Err(e),
                            Ok(_) => return Ok(()),
                        };
                        
                    }else{
                        match self.policies.get(id -1) {
                            None => return Err(Error::NotExistedPolicy),
                            Some(p) if p.is_canceled == false => return Err(Error::PreviousPolicyNotCancelled),
                            Some(_p) => {
                                match self.execute_cancel(&mut v){
                                    Err(e) => return Err(e),
                                    Ok(_) => return Ok(()),
                                };
                            }
                        }
                    }
                }
            }
        }

        fn do_policy_cancel(&mut self, policy: &mut PolicyInfo, caller: AccountId) -> Result<()>{
            self.total_coverage -= policy.coverage;
            self.acc_sps_down += policy.delta_acc_sps;
            policy.is_canceled = true;
            self.latest_unfrozen_index = policy.latest_provider_index;
            self.update_k_last_by_cancel(self.total_coverage);

            match self.erc20.transfer(caller, policy.deposit){
                Err(e) if e == Erc20Error::InsufficientAllowance => return Err(Error::InsufficientAllowance),
                Err(e) if e == Erc20Error::InsufficientBalance => return Err(Error::InsufficientBalance),
                Err(_e) => return Err(Error::TransferError),
                Ok(_) => return Ok(()),
            }

        }


        fn update_k_last_by_cancel(&mut self, total_coverage: Balance){
            if self.token_staked_here > total_coverage {
                let useable_capital = self.token_staked_here - total_coverage;
                let tentative_fee = self.k_last / (useable_capital + self.virtual_param);
                if tentative_fee >= self.min_fee {
                    ()
                }else{
                    self.k_last = self.min_fee * (useable_capital + self.virtual_param);
                }
            }else{
                ()
            }
        }

        fn execute_cancel(&mut self, policy: &mut PolicyInfo) -> Result<()> {
            let today = self.env().block_timestamp();
            if policy.effective_until > today {
                return Err(Error::NotExpiredPolicy);
            } else if policy.in_claim_applying == true {
                return Err(Error::ClaimingInProgress);
            } else{
                let time_pass = today - policy.effective_until;
                if time_pass <= 86_400_000 {
                    let caller = self.env().caller();
                    if caller != policy.beneficiary {
                        return Err(Error::OnlyPolicyHolderCanCancel);
                    }else{
                        match self.do_policy_cancel(policy, caller){
                            Err(e) => return Err(e),
                            Ok(_) => return Ok(()),
                        }
                    }
                } else{
                        let caller = self.env().caller();
                        match self.do_policy_cancel(policy, caller){
                            Err(e) => return Err(e),
                            Ok(_) => return Ok(()),
                        }
                }
            }
        }

        #[ink(message)]
        pub fn policy_claim_apply(&mut self, id: u128) -> Result<()> {
            let caller = self.env().caller();
            let today = self.env().block_timestamp();
            match self.policies.get(id) {
                None => return Err(Error::NotExistedPolicy),
                Some(p) if p.beneficiary != caller => return Err(Error::NotBeneficiary),
                Some(p) if p.is_claimed == true => return Err(Error::AlreadyClaimedPolicy),
                Some(p) if p.in_claim_applying == true => return Err(Error::InClaimingProgress),
                Some(p) if p.is_canceled == true => return Err(Error:: AlreadyCancelledPolicy),
                Some(p) if today > p.effective_until => return Err(Error::NotEffectivePolicy),
                Some(mut p) => {
                    p.in_claim_applying = true;
                    return Ok(());
                }
            }
        }
        
        #[ink(message)]
        pub fn refuse_apply(&mut self, id: u128) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.judger {
                return Err(Error::NotJudger);
            }else{
                match self.policies.get(id) {
                    None => return Err(Error::NotExistedPolicy),
                    Some(mut p) => {
                        p.in_claim_applying = false;
                        return Ok(());
                    }
                }
            }
        }

        #[ink(message)]
        pub fn accept_apply(&mut self, id: u128) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.judger {
                return Err(Error::NotJudger);
            }else{
                match self.policies.get(id) {
                    None => return Err(Error::NotExistedPolicy),
                    Some(p) if p.in_claim_applying == false => return Err(Error::NotInClaimingProgress),
                    Some(mut p) => {
                        if self.erc20.balance_of(self.risk_reserve) >= p.coverage {
                            match self.erc20.transfer_from(self.risk_reserve, p.beneficiary, p.coverage){
                                Err(e) if e == Erc20Error::InsufficientAllowance => return Err(Error::InsufficientAllowance),
                                Err(e) if e == Erc20Error::InsufficientBalance => return Err(Error::InsufficientBalance),
                                Err(_e) => return Err(Error::TransferError),
                                Ok(_) => {
                                    p.in_claim_applying = false;
                                    p.is_claimed = true;
                                    return Ok(())},
                            }
                        } else {
                            match self.erc20.transfer_from(self.risk_reserve, p.beneficiary, self.erc20.balance_of(self.risk_reserve)){
                                Err(e) if e == Erc20Error::InsufficientAllowance => return Err(Error::InsufficientAllowance),
                                Err(e) if e == Erc20Error::InsufficientBalance => return Err(Error::InsufficientBalance),
                                Err(_e) => return Err(Error::TransferError),
                                Ok(_) => {
                                    p.in_claim_applying = false;
                                    p.is_claimed = true;
                                    let exceeded = p.coverage - self.erc20.balance_of(self.risk_reserve);
                                    match self.exceeded_pay(p.beneficiary, exceeded){
                                        Err(e) => return Err(e),
                                        Ok(_) => return Ok(()),
                                    }
                                },
                            }
                            
                            
                        }

                    }
                }
            }
        }


        fn exceeded_pay(&mut self, to: AccountId, exceeded: Balance) -> Result<()> {
            let pre_reserve = self.token_staked_here + self.token_frozen_here;
            let after_reserve = pre_reserve - exceeded;

            let delta_rate = after_reserve * 100_000 / pre_reserve;

            self.exchange_rate = self.exchange_rate * delta_rate / 100_000;

            self.token_staked_here = self.token_staked_here * delta_rate / 100_000;

            self.token_frozen_here = self.token_frozen_here * delta_rate / 100_000;

            match self.erc20.transfer(to, exceeded) {
                Err(e) if e == Erc20Error::InsufficientAllowance => return Err(Error::InsufficientAllowance),
                Err(e) if e == Erc20Error::InsufficientBalance => return Err(Error::InsufficientBalance),
                Err(_e) => return Err(Error::TransferError),
                Ok(_) => return Ok(()),
            }
        }

        #[ink(message)]
        pub fn unused_capital_for_mining(&mut self, amount: Balance, to: AccountId) -> Result<()> {
            let caller = self.env().caller();
            if caller != self.judger {
                return Err(Error::NotJudger);
            }else {
                match self.is_valid_mining_proxy.get(to) {
                    None => return Err(Error::NotValidMiningProxy),
                    Some(v) if v == false => return Err(Error::NotValidMiningProxy),
                    Some(_v) => {
                        match self.erc20.transfer(to, amount) {
                            Err(e) if e == Erc20Error::InsufficientAllowance => return Err(Error::InsufficientAllowance),
                            Err(e) if e == Erc20Error::InsufficientBalance => return Err(Error::InsufficientBalance),
                            Err(_e) => return Err(Error::TransferError),
                            Ok(_) => return Ok(()),
                        }
                    }
                }
            }
            
        }


        #[ink(message)]
        pub fn check_judger(&self) -> AccountId{
            self.judger
        }

        #[ink(message)]
        pub fn check_official(&self) -> AccountId{
            self.official
        }



        // #[ink(message)]
        // pub fn get(&self) ->  Balance{
        //     self.erc20.total_supply()        
        // }


        // #[ink(message)]
        // pub fn balance_of(&self, owner: AccountId) -> Balance {
        //     self.erc20.balance_of(owner)
        // }

        // #[ink(message)]
        // pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
        //     self.erc20.allowance(owner, spender)
        // }


        // fn transfer(&mut self, to: AccountId, amount: Balance) {
        //     self.erc20.transfer(to, amount);
        // }

        // #[ink(message)]
        // pub fn approve(&mut self, 
        //     spender: AccountId, value: Balance){
        //     self.erc20.approve(spender, value);   
        // }

        // pub fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance ) {
        //     self.transfer_from(from, to, value);
        // }

    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink::codegen::Env;

        #[ink::test]
        fn judger_transfer_should_works() {

            let mut meta_defender = create_default();


            let accounts = default_accounts();
            let alice  = accounts.alice;
            let bob  = accounts.bob;
            let charlie  = accounts.charlie;

            // current judger is charlie, 
            // charlie initiates the judger transfer, should work
            set_sender(charlie);
            meta_defender.judger_transfer(alice);
            assert_eq!(meta_defender.check_judger(), alice);

        }

        #[ink::test]
        #[should_panic]
        fn judger_transfer_should_not_works() {

            let mut meta_defender = create_default();


            let accounts = default_accounts();
            let alice  = accounts.alice;
            let bob  = accounts.bob;
            let charlie  = accounts.charlie;


            set_sender(bob);
            // current judger is charlie, 
            // bob initiates the judger transfer, should not work
            meta_defender.judger_transfer(alice);
            assert_eq!(meta_defender.check_judger(), alice);
        }

        // #[ink::test]
        // fn provide_capital_should_work() {

        //     let accounts = default_accounts();
        //     let alice = accounts.alice;
        //     let bob  = accounts.bob;
        //     let charlie  = accounts.charlie;
        //     let django  = accounts.django;

            
        //     set_sender(bob);
        //     let mut erc20 = create_erc20(10000000);
        //     let erc20_account = erc20.env().account_id();
        //     let mut meta_defender = create_meta_defender(bob, charlie, django, 10_000_000, erc20_account);

        //     set_sender(alice);
        //     erc20.transfer(bob, 2000);
        //     println!("{}", erc20.balance_of(bob));

        //     set_sender(alice);
        //     meta_defender.provide_capital(2000);
        //     println!("{}", meta_defender.get_useable_capital());
            
        // }

        fn create_default() -> MetaDefender{

            let accounts = default_accounts();
            let alice = accounts.alice;
            let bob  = accounts.bob;
            let charlie  = accounts.charlie;
            let django  = accounts.django;

            
            set_sender(alice);
            let mut erc20 = create_erc20(10000000);
            let erc20_account = erc20.env().account_id();
            let meta_defender = create_meta_defender(bob, charlie, django, 10_000_000, erc20_account);

            set_sender(alice);
            erc20.transfer(bob, 2000);
            println!("{}", erc20.balance_of(bob));
            meta_defender
        }

        fn create_erc20(initial_balance: Balance) -> Erc20{
            let accounts = default_accounts();
            set_sender(accounts.alice);
            Erc20::new(initial_balance)
        }

        fn create_meta_defender(official: AccountId, judger: AccountId, risk_reserve: AccountId, virtual_param: u128, erc20: AccountId) -> MetaDefender{
            let accounts = default_accounts();
            set_sender(accounts.alice);
            MetaDefender::new(official, judger, risk_reserve, virtual_param, erc20)
        }

        fn contract_id() -> AccountId {
            ink::env::test::callee::<ink::env::DefaultEnvironment>()
        }

        fn set_sender(sender: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
        }

        fn default_accounts(
        ) -> ink::env::test::DefaultAccounts<ink::env::DefaultEnvironment> {
            ink::env::test::default_accounts::<ink::env::DefaultEnvironment>()
        }

        fn set_balance(account_id: AccountId, balance: Balance) {
            ink::env::test::set_account_balance::<ink::env::DefaultEnvironment>(
                account_id, balance,
            )
        }

        fn get_balance(account_id: AccountId) -> Balance {
            ink::env::test::get_account_balance::<ink::env::DefaultEnvironment>(
                account_id,
            )
            .expect("Cannot get account balance")
        }

    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::MetaDefenderRef;
        use ink_e2e::build_message;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
    
        #[ink_e2e::test(
            additional_contracts = "erc20/Cargo.toml"
        )]
        async fn e2e_delegator(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {

            let erc20_constructor = Erc20Ref::new(
                10_000
            );

            let erc20_acc_id = client
            .instantiate("erc20", &mut ink_e2e::alice(), erc20_constructor, 0, None)
            .await
            .expect("instantiate failed")
            .account_id;


            let md_constructor = MetaDefenderRef::new(
                ink_e2e::alice(),
                ink_e2e::alice(),
                ink_e2e::alice(),
                100_000,
                erc20_acc_id
            );

            let md_acc_id = client
                .instantiate("meta_defender", &mut ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let transfer = build_message::<Erc20Ref>(erc20_acc_id.clone())
                .call(|contract| contract.transfer(ink_e2e::bob(), 2000));


            let erc20_acc_id = client
                .instantiate("delegator", &mut ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;
            Ok(())
        }
    
    }
}
    
