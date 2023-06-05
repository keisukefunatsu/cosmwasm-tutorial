use crate::msg::{AdminListResp, ExecuteMsg, GreetResp, InstantiateMsg, QueryMsg};
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
};

use crate::state::ADMINS;

pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    let admins: StdResult<Vec<_>> = _msg
        .admins
        .into_iter()
        .map(|addr| _deps.api.addr_validate(&addr))
        .collect();
    ADMINS.save(_deps.storage, &admins?)?;
    Ok(Response::new())
}

pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    // println!("{:?}", msg);
    match msg {
        Greet {} => to_binary(&query::greet()?),
        AdminsList {} => to_binary(&query::admins_list(_deps)?),
    }
}

mod query {
    use super::*;

    pub fn greet() -> StdResult<GreetResp> {
        let resp = GreetResp {
            message: "Hello World".to_owned(),
        };

        Ok(resp)
    }

    pub fn admins_list(deps: Deps) -> StdResult<AdminListResp> {
        let admins = ADMINS.load(deps.storage)?;
        let resp = AdminListResp { admins };
        Ok(resp)
    }
}

use crate::error::ContractError;
#[allow(unused_imports)]
use crate::msg::*;

pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;
    match msg {
        AddMembers { admins } => exec::add_members(deps, info, admins),
        Leave {} => exec::leave(deps, info),
    }
}

mod exec {
    use super::*;
    pub fn add_members(
        deps: DepsMut,
        info: MessageInfo,
        admins: Vec<String>,
    ) -> Result<Response, ContractError> {
        let mut current_admins = ADMINS.load(deps.storage)?;
        if !current_admins.contains(&info.sender) {
            return Err(ContractError::Unauthorized {
                sender: info.sender,
            });
        }

        let events = admins
            .iter()
            .map(|admin| Event::new("admin_added").add_attribute("addr", admin));
        let resp = Response::new()
            .add_events(events)
            .add_attribute("action", "add_members")
            .add_attribute("add_count", admins.len().to_string());
        let admins: StdResult<Vec<_>> = admins
            .into_iter()
            .map(|addr| deps.api.addr_validate(&addr))
            .collect();
        current_admins.append(&mut admins?);
        let _ = ADMINS.save(deps.storage, &current_admins);
        Ok(resp)
    }

    pub fn leave(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
        let _ = ADMINS.update(deps.storage, move |admins| -> StdResult<_> {
            let admins = admins
                .into_iter()
                .filter(|admin| *admin != info.sender)
                .collect();
            Ok(admins)
        });
        Ok(Response::new())
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use cw_multi_test::{App, ContractWrapper, Executor};

    use super::*;

    #[test]
    fn instantiation() {
        let mut app = App::default();

        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { admins: vec![] },
                &[],
                "Contract",
                None,
            )
            .unwrap();
        let resp: AdminListResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::AdminsList {})
            .unwrap();
        assert_eq!(resp, AdminListResp { admins: vec![] });

        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg {
                    admins: vec!["admin1".to_owned(), "admin2".to_owned()],
                },
                &[],
                "Contract",
                None,
            )
            .unwrap();
        let resp: AdminListResp = app
            .wrap()
            .query_wasm_smart(addr, &QueryMsg::AdminsList {})
            .unwrap();
        assert_eq!(
            resp,
            AdminListResp {
                admins: vec![Addr::unchecked("admin1"), Addr::unchecked("admin2")]
            }
        );
    }

    #[test]
    fn unauthorized() {
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));
        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg { admins: vec![] },
                &[],
                "Contract",
                None,
            )
            .unwrap();
        let err = app
            .execute_contract(
                Addr::unchecked("user"),
                addr,
                &ExecuteMsg::AddMembers {
                    admins: vec!["user".to_owned()],
                },
                &[],
            )
            .unwrap_err();
        assert_eq!(
            ContractError::Unauthorized {
                sender: Addr::unchecked("user")
            },
            err.downcast().unwrap()
        );
    }
    #[test]
    fn add_members() {
        let mut app = App::default();
        let code = ContractWrapper::new(execute, instantiate, query);
        let code_id = app.store_code(Box::new(code));
        let addr = app
            .instantiate_contract(
                code_id,
                Addr::unchecked("owner"),
                &InstantiateMsg {
                    admins: vec!["owner".to_owned()],
                },
                &[],
                "Contract",
                None,
            )
            .unwrap();
        let resp = app
            .execute_contract(
                Addr::unchecked("owner"),
                addr,
                &ExecuteMsg::AddMembers {
                    admins: vec!["member1".to_owned()],
                },
                &[],
            )
            .unwrap();
        let wasm = resp.events.iter().find(|ev| ev.ty == "wasm").unwrap();

        assert_eq!(
            wasm.attributes
                .iter()
                .find(|attr| attr.key == "action")
                .unwrap()
                .value,
            "add_members"
        );
        assert_eq!(
            wasm.attributes
                .iter()
                .find(|attr| attr.key == "add_count")
                .unwrap()
                .value,
            "1"
        );

        let admin_added: Vec<_> = resp
            .events
            .iter()
            .filter(|ev| ev.ty == "wasm-admin_added")
            .collect();

        assert_eq!(admin_added.len(), 1);

        assert_eq!(
            admin_added[0]
                .attributes
                .iter()
                .find(|attr| attr.key == "addr")
                .unwrap()
                .value,
            "member1"
        )
    }
}
