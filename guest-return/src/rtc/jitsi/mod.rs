// Copyright 2024 Amagi Poland
pub mod error;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub use self::error::Error;
#[derive(Serialize, Deserialize,Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListInvited {
    code: u32,
    user_message: String,
    success: String,
    rooms: Room,
}

#[derive(Serialize, Deserialize,Debug)]
#[serde(rename_all = "camelCase")]
pub struct Room {
    invitation_map: HashMap<String, RoomEntry>,
}

#[derive(Serialize, Deserialize,Debug,Clone)]
#[serde(rename_all = "camelCase")]
pub struct RoomEntry {
    pretty_name: String,
    is_non_deletable: bool,
    is_no_return: bool,
    invitations: Vec<Invitation>,
}

impl RoomEntry {
  pub fn is_return_enabled(&mut self) -> bool {
    return !self.is_no_return;
  }
}

#[derive(Serialize, Deserialize,Debug,Clone)]
#[serde(rename_all = "camelCase")]
pub struct Invitation {
    #[serde(default)]
    link: String,

    #[serde(default)]
    username: String,

    #[serde(default)]
    expiration: u64,

    #[serde(default)]
    conference_name: String,

    #[serde(default)]
    pretty_conference_name: String,

    #[serde(default)]
    high_quality_audio: bool,

    #[serde(default)]
    source_type: String,

    #[serde(default)]
    video_quality: String,

    #[serde(default)]
    incoming_video_quality: String,

    #[serde(default)]
    shortened_id: String,

    #[serde(default)]
    endpoint_id: String,

    #[serde(default)]
    recording_disabled: bool,

    #[serde(default)]
    webcam_enabled: bool,

    #[serde(default)]
    microphone_enabled: bool,

    #[serde(default)]
    screen_share_enabled: bool,

    #[serde(default)]
    restrictions: Vec<String>,
}

pub struct Jitsi {
    http_client: reqwest::Client,
    env: String,
    lsp_id: u32,
}

impl Jitsi {
    pub fn new(env: String, lsp_id: u32) -> Jitsi {
        return Jitsi {
          http_client: reqwest::Client::new(),
          env: env,
          lsp_id: lsp_id,
        }
    }

    pub fn from_str(&mut self, json: String) -> serde_json::Result<ListInvited> {
        // in case we want to convert type of the returned Error here,
        // see https://stackoverflow.com/questions/57794849/result-getting-unexpected-type-argument
        let parsed = match serde_json::from_str::<ListInvited>(&json) {
            Ok(json) => json,
            Err(e) => return Err(e),
        };

        return Ok(parsed);
    }

    pub async fn get_lsp_guest_details(&mut self,) -> Result<HashMap<String, RoomEntry>, Box<dyn std::error::Error>> {
        match self.http_client.get(self.get_url_list_invited()?).send().await?.json::<ListInvited>().await { 
            Ok(json) => Ok(json.rooms.invitation_map),
            Err(e) => return Err(Box::new(e)),
        }
    }

    pub async fn get_room_details(&mut self, room_id: &str) -> Result<RoomEntry, Box<dyn std::error::Error>> {
        let list = match self.http_client.get(self.get_url_list_invited()?).send().await?.json::<ListInvited>().await { 
            Ok(json) => json,
            Err(e) => return Err(Box::new(e)),
        };

        for (e_room_id, room) in list.rooms.invitation_map.iter() {
            if e_room_id.eq(&room_id) {
                return Ok(room.clone());
            }
        }

        Err(Box::new(Error::new("room not found")))
    }

    fn get_url_list_invited(&self) -> Result<String, Box<dyn std::error::Error>> {
      match self.env.to_lowercase().as_str() {
        "dev" => Ok(format!("https://brutus.tellyo.com:8443/tellyo-rtc-web/rest/lsp/jitsi/listInvited/{}", self.lsp_id)),
        "qa" => Ok(format!("https://rtc-elb-qa.tellyo.com/tellyo-rtc-web/rest/lsp/jitsi/listInvited/{}", self.lsp_id)),
        _ => Err(Box::new(Error::new("invalid ENV. Possible values: [DEV, QA]")))
      }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static JSON_SAMPLE_INPUT: &'static str = r#"{
  "code": 0,
  "userMessage": "",
  "success": "ok",
  "rooms": {
    "invitationMap": {
      "main-k4qltpl5e1nw": {
        "prettyName": "main",
        "isNonDeletable": false,
        "isNoReturn": false,
        "invitations": [
          {
            "link": "https://conference-dev.tellyo.com/client?token=4epPhgWOqL8",
            "username": "Alek",
            "expiration": 10413788400000,
            "conferenceName": "main-k4qltpl5e1nw",
            "prettyConferenceName": "main",
            "highQualityAudio": false,
            "sourceType": "WEBCAM",
            "videoQuality": "HIGHEST",
            "incomingVideoQuality": "HIGHEST",
            "shortenedId": "4epPhgWOqL8",
            "endpointId": "a7e834b4",
            "recordingDisabled": false,
            "webcamEnabled": false,
            "microphoneEnabled": false,
            "screenShareEnabled": false,
            "restrictions": [
              "ON_AIR"
            ]
          },
          {
            "link": "https://conference-dev.tellyo.com/client?token=E3Yai5A7XGr",
            "username": "Mateusz",
            "expiration": 10413788400000,
            "conferenceName": "main-k4qltpl5e1nw",
            "prettyConferenceName": "main",
            "highQualityAudio": false,
            "sourceType": "WEBCAM",
            "videoQuality": "HIGHEST",
            "incomingVideoQuality": "HIGHEST",
            "shortenedId": "E3Yai5A7XGr",
            "endpointId": "96db0dbc",
            "recordingDisabled": false,
            "webcamEnabled": true,
            "microphoneEnabled": false,
            "screenShareEnabled": false,
            "restrictions": [
              "ON_AIR"
            ]
          },
          {
            "link": "https://conference-dev.tellyo.com/client?token=K577seEqNzD",
            "username": "Jan",
            "expiration": 10413788400000,
            "conferenceName": "main-k4qltpl5e1nw",
            "prettyConferenceName": "main",
            "highQualityAudio": false,
            "sourceType": "WEBCAM",
            "videoQuality": "HIGHEST",
            "incomingVideoQuality": "HIGHEST",
            "shortenedId": "K577seEqNzD",
            "endpointId": "03ef8672",
            "recordingDisabled": false,
            "webcamEnabled": true,
            "microphoneEnabled": true,
            "screenShareEnabled": false,
            "restrictions": [
              "ON_AIR",
              "SCREEN_SHARING"
            ]
          },
          {
            "link": "https://conference-dev.tellyo.com/client?token=ZbqXhwlqYaD",
            "username": "Aleek Bulldozer",
            "expiration": 10413788400000,
            "conferenceName": "main-k4qltpl5e1nw",
            "prettyConferenceName": "main",
            "highQualityAudio": false,
            "sourceType": "WEBCAM",
            "videoQuality": "HIGHEST",
            "incomingVideoQuality": "HIGHEST",
            "shortenedId": "ZbqXhwlqYaD",
            "endpointId": "e0ee2ba3",
            "recordingDisabled": false,
            "webcamEnabled": true,
            "microphoneEnabled": false,
            "screenShareEnabled": false,
            "restrictions": [
              "ON_AIR"
            ]
          }
        ]
      }
    },
    "maxRooms": 3
  }
}"#;

    #[test]
    fn test_from_str() {
        let mut j = Jitsi::new("DEV".to_string(), 1516);
        let result = j.from_str(JSON_SAMPLE_INPUT.to_string());
        println!("{:?}", result);
        assert_eq!(result.is_err(), false);
    }

    fn test_map() {
        let mut j = Jitsi::new("DEV".to_string(), 1516);
        let result = j.from_str(JSON_SAMPLE_INPUT.to_string());
        println!("{:?}", result);
        assert_eq!(result.is_err(), false);

        //let list = result.unwrap();
    }


}