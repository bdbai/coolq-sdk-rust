//! 在获取陌生人信息和好友列表时使用
//!
//!
//! 权限分组请看[Authority]。算是一个小小的权限管理吧
//!
//! 使用[`add_master`]和[`add_super_admin`]来添加主人和管理员。
//!
//! 使用[`check_authority`]来检查用户权限。
//!
//! [Authorit]: Authority
//! [`add_master`]: User::add_master
//! [`add_super_admin`]: User::add_super_admin
//! [`check_authority`]: Authority::check_authority

use std::{convert::TryInto, io::Cursor, sync::RwLock};

use byteorder::{BigEndian, ReadBytesExt};

use crate::{
    api::{get_stranger_info, send_private_msg, Convert},
    targets::{
        group::{GroupMember, GroupRole},
        message::SendMessage,
        ReadString,
    },
};

lazy_static! {
    static ref MasterList: RwLock<Vec<i64>> = RwLock::new(Vec::new());
    static ref SuperAdminList: RwLock<Vec<i64>> = RwLock::new(Vec::new());
}

#[derive(Debug, Clone)]
pub enum UserSex {
    Male,
    Female,
    Unknown,
}

impl From<i32> for UserSex {
    fn from(i: i32) -> Self {
        match i {
            0 => UserSex::Male,
            1 => UserSex::Female,
            _ => UserSex::Unknown,
        }
    }
}

impl Default for UserSex {
    fn default() -> Self {
        UserSex::Unknown
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Copy)]
pub enum Authority {
    Master = 0,
    SuperAdmin = 1,
    GroupOwner = 2,
    GroupAdmin = 3,
    User = 4,
}

impl Authority {
    pub fn check_authority(&self, authority: Authority) -> bool {
        self <= &authority
    }

    pub fn new(id: i64) -> Authority {
        if !SuperAdminList
            .read()
            .expect("cannot read SuperAdminList")
            .iter()
            .all(|qq| *qq != id)
        {
            Authority::SuperAdmin
        } else if !MasterList
            .read()
            .expect("cannot read MasterList")
            .iter()
            .all(|qq| *qq != id)
        {
            Authority::Master
        } else {
            Authority::User
        }
    }

    pub(crate) fn from_group_member(gm: &GroupMember) -> Authority {
        let authority = Authority::new(gm.user_id);
        if authority != Authority::User {
            authority
        } else {
            match gm.role {
                GroupRole::Member => Authority::User,
                GroupRole::Admin => Authority::GroupAdmin,
                GroupRole::Owner => Authority::GroupOwner,
            }
        }
    }
}

impl Default for Authority {
    fn default() -> Self {
        Authority::User
    }
}

/// get_friend_list
#[derive(Debug, Clone)]
pub struct FriendInfo {
    pub user_id: i64,
    pub nickname: String,
    pub remark: String,
}

impl FriendInfo {
    pub(crate) fn decode(b: &[u8]) -> std::io::Result<FriendInfo> {
        let mut b = Cursor::new(&b);
        Ok(FriendInfo {
            user_id: b.read_i64::<BigEndian>()?,
            nickname: b.read_string()?,
            remark: b.read_string()?,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct User {
    pub user_id: i64,
    pub nickname: String,
    pub sex: UserSex,
    pub age: i32,
    pub authority: Authority,
}

impl SendMessage for User {
    fn send(&self, msg: impl ToString) -> crate::api::Result<Convert<i32>> {
        send_private_msg(self.user_id, msg.to_string())
    }
}

impl User {
    pub fn add_master(user_id: i64) {
        MasterList.write().unwrap().push(user_id);
    }

    pub fn add_super_admin(user_id: i64) {
        SuperAdminList.write().unwrap().push(user_id);
    }

    pub fn get_masters() -> Vec<i64> {
        MasterList.read().unwrap().clone()
    }

    pub fn get_super_admins() -> Vec<i64> {
        SuperAdminList.read().unwrap().clone()
    }

    //为了防止获取频率过大，所有从事件获取到的User皆是从缓存取的。
    //如果想获得最新信息，请使用update。
    pub(crate) fn new(user_id: i64) -> User {
        let mut user: User = if let Ok(c) = get_stranger_info(user_id, false) {
            c.try_into().expect("cannot decode User")
        } else {
            Default::default()
        };
        user.user_id = user_id;
        user.set_authority(Authority::new(user_id));
        user
    }

    pub(crate) fn set_authority(&mut self, authority: Authority) {
        if !self.authority.check_authority(authority) {
            self.authority = authority
        }
    }

    pub fn update(&mut self) -> crate::api::Result<User> {
        Ok(get_stranger_info(self.user_id, true)?
            .try_into()
            .expect("cannot decode User"))
    }

    pub(crate) fn decode(b: &[u8]) -> std::io::Result<User> {
        dbg!(b);
        let mut b = Cursor::new(base64::decode(&b).expect("Invalid base64 - decode User"));
        Ok(User {
            user_id: b.read_i64::<BigEndian>()?,
            nickname: b.read_string()?,
            sex: UserSex::from(b.read_i32::<BigEndian>()?),
            age: b.read_i32::<BigEndian>()?,
            authority: Authority::User,
        })
    }
}
