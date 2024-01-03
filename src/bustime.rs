extern crate reqwest;
extern crate serde;
extern crate serde_json;

use crate::result;

pub struct BusTimeDisplayData {
    pub uptown_waits: Vec<i64>,
    pub downtown_waits: Vec<i64>,
    pub timestamp: time::OffsetDateTime,
}

#[derive(Debug)]
pub struct GarfieldBusArrivals {
    pub uptown_timestamps: Vec<time::OffsetDateTime>,
    pub downtown_timestamps: Vec<time::OffsetDateTime>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MTABusTimeRoot {
    siri: MTASiri,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MTASiri {
    service_delivery: MTAServiceDelivery,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MTAServiceDelivery {
    response_timestamp: String,
    stop_monitoring_delivery: Vec<MTAStopMonitoringDelivery>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MTAStopMonitoringDelivery {
    monitored_stop_visit: Vec<MTAMonitoredStopVisit>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MTAMonitoredStopVisit {
    monitored_vehicle_journey: MTAMonitoredVehicleJourney,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MTAMonitoredVehicleJourney {
    line_ref: String,
    vehicle_ref: String,
    monitored_call: MTAMonitoredCall,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct MTAMonitoredCall {
    aimed_arrival_time: Option<String>,
    expected_arrival_time: Option<String>,
    aimed_departure_time: Option<String>,
    expected_departure_time: Option<String>,
//    #[serde(with = "time::serde::rfc3339::option")]
//    expected_departure_time: Option<time::OffsetDateTime>,
}

pub fn get_garfield_bus_arrivals(api_key: &str) -> result::TTDashResult<GarfieldBusArrivals> {
    let downtown_url = format!("https://bustime.mta.info/api/siri/stop-monitoring.json?key={api_key}&OperatorRef=MTA&MonitoringRef=308215&LineRef=MTA%20NYCT_B63", api_key = api_key);

    let uptown_url = format!("https://bustime.mta.info/api/siri/stop-monitoring.json?key={api_key}&OperatorRef=MTA&MonitoringRef=308208&LineRef=MTA%20NYCT_B63", api_key = api_key);

    return Ok(GarfieldBusArrivals{
        uptown_timestamps: fetch_and_parse(&uptown_url)?,
        downtown_timestamps: fetch_and_parse(&downtown_url)?,
    });
}

fn fetch_and_parse(url: &str) -> result::TTDashResult<Vec<time::OffsetDateTime>> {
    debug!("Fetching {}", url);
    let mut response = reqwest::blocking::get(url)?;
    let mut response_body = String::new();
    use std::io::Read;
    response.read_to_string(&mut response_body)?;

    return parse_response(&response_body);
}

fn parse_response(response_body: &str) -> result::TTDashResult<Vec<time::OffsetDateTime>> {
    let response_json: MTABusTimeRoot = serde_json::from_str(&response_body)?;

    assert_eq!(1, response_json.siri.service_delivery.stop_monitoring_delivery.len());

    return Ok(response_json.siri.service_delivery.stop_monitoring_delivery[0].monitored_stop_visit.iter().map(|ref msv| {
        let call = &msv.monitored_vehicle_journey.monitored_call;
        if call.expected_arrival_time.is_some() {
            return time::OffsetDateTime::parse(
                call.expected_arrival_time.clone().unwrap().as_ref(),
                &time::format_description::well_known::Rfc3339).expect("parsing date");
        }
        return time::OffsetDateTime::parse(
            call.aimed_arrival_time.clone().unwrap().as_ref(),
            &time::format_description::well_known::Rfc3339).expect("parsing date");
    }).collect());
}

#[cfg(test)]
mod tests {

    extern crate serde_json;
    extern crate time;

    #[test]
    fn parse_json() {
        let raw_json = r#"{"Siri":{"ServiceDelivery":{"ResponseTimestamp":"2024-01-02T16:28:45.691-05:00","StopMonitoringDelivery":[{"MonitoredStopVisit":[{"MonitoredVehicleJourney":{"LineRef":"MTA NYCT_B63","DirectionRef":"1","FramedVehicleJourneyRef":{"DataFrameRef":"2024-01-02","DatedVehicleJourneyRef":"MTA NYCT_JG_D3-Weekday-SDon-097100_B63_659"},"JourneyPatternRef":"MTA_B630027","PublishedLineName":"B63","OperatorRef":"MTA NYCT","OriginRef":"MTA_901601","DestinationName":"BAY RIDGE SHORE RD via 5 AV","SituationRef":[{"SituationSimpleRef":"MTA NYCT_lmm:planned_work:9418"}],"Monitored":true,"VehicleLocation":{"Longitude":-73.977535,"Latitude":40.68405},"Bearing":336.80142,"ProgressRate":"normalProgress","BlockRef":"MTA NYCT_JG_D3-Weekday-SDon_E_JG_38340_B63-672","VehicleRef":"MTA NYCT_438","MonitoredCall":{"AimedArrivalTime":"2024-01-02T16:38:22.000-05:00","ExpectedArrivalTime":"2024-01-02T16:38:08.174-05:00","AimedDepartureTime":"2024-01-02T16:38:22.000-05:00","ExpectedDepartureTime":"2024-01-02T16:38:08.174-05:00","Extensions":{"Distances":{"PresentableDistance":"0.8 miles away","DistanceFromCall":1226.87,"StopsFromCall":4,"CallDistanceAlongRoute":3556.08},"VehicleFeatures":{"StrollerVehicle":false}},"StopPointRef":"MTA_308215","VisitNumber":1,"StopPointName":"5 AV/GARFIELD PL"},"OnwardCalls":{}},"RecordedAtTime":"2024-01-02T16:28:37.000-05:00"},{"MonitoredVehicleJourney":{"LineRef":"MTA NYCT_B63","DirectionRef":"1","FramedVehicleJourneyRef":{"DataFrameRef":"2024-01-02","DatedVehicleJourneyRef":"MTA NYCT_JG_D3-Weekday-SDon-098000_B63_675"},"JourneyPatternRef":"MTA_B630027","PublishedLineName":"B63","OperatorRef":"MTA NYCT","OriginRef":"MTA_901601","DestinationName":"BAY RIDGE SHORE RD via 5 AV","SituationRef":[{"SituationSimpleRef":"MTA NYCT_lmm:planned_work:9418"}],"Monitored":true,"VehicleLocation":{"Longitude":-73.988897,"Latitude":40.688503},"Bearing":338.19858,"ProgressRate":"normalProgress","BlockRef":"MTA NYCT_JG_D3-Weekday-SDon_E_JG_26520_B63-666","VehicleRef":"MTA NYCT_764","MonitoredCall":{"AimedArrivalTime":"2024-01-02T16:47:22.000-05:00","ExpectedArrivalTime":"2024-01-02T16:46:57.163-05:00","AimedDepartureTime":"2024-01-02T16:47:22.000-05:00","ExpectedDepartureTime":"2024-01-02T16:46:57.163-05:00","Extensions":{"Distances":{"PresentableDistance":"1.4 miles away","DistanceFromCall":2305.3,"StopsFromCall":10,"CallDistanceAlongRoute":3556.08},"VehicleFeatures":{"StrollerVehicle":false}},"StopPointRef":"MTA_308215","VisitNumber":1,"StopPointName":"5 AV/GARFIELD PL"},"OnwardCalls":{}},"RecordedAtTime":"2024-01-02T16:28:40.000-05:00"},{"MonitoredVehicleJourney":{"LineRef":"MTA NYCT_B63","DirectionRef":"1","FramedVehicleJourneyRef":{"DataFrameRef":"2024-01-02","DatedVehicleJourneyRef":"MTA NYCT_JG_D3-Weekday-SDon-099800_B63_669"},"JourneyPatternRef":"MTA_B630027","PublishedLineName":"B63","OperatorRef":"MTA NYCT","OriginRef":"MTA_901601","DestinationName":"BAY RIDGE SHORE RD via 5 AV","OriginAimedDepartureTime":"2024-01-02T16:38:00.000-05:00","SituationRef":[{"SituationSimpleRef":"MTA NYCT_lmm:planned_work:9418"}],"Monitored":true,"VehicleLocation":{"Longitude":-73.996122,"Latitude":40.690716},"Bearing":164.17384,"ProgressRate":"normalProgress","ProgressStatus":"prevTrip","BlockRef":"MTA NYCT_JG_D3-Weekday-SDon_E_JG_18600_B63-655","VehicleRef":"MTA NYCT_393","MonitoredCall":{"AimedArrivalTime":"2024-01-02T17:05:22.000-05:00","ExpectedArrivalTime":"2024-01-02T17:03:57.626-05:00","AimedDepartureTime":"2024-01-02T17:05:22.000-05:00","ExpectedDepartureTime":"2024-01-02T17:03:57.626-05:00","Extensions":{"Distances":{"PresentableDistance":"2.6 miles away","DistanceFromCall":4125.86,"StopsFromCall":15,"CallDistanceAlongRoute":3556.08},"VehicleFeatures":{"StrollerVehicle":false}},"StopPointRef":"MTA_308215","VisitNumber":1,"StopPointName":"5 AV/GARFIELD PL"},"OnwardCalls":{}},"RecordedAtTime":"2024-01-02T16:28:36.000-05:00"},{"MonitoredVehicleJourney":{"LineRef":"MTA NYCT_B63","DirectionRef":"1","FramedVehicleJourneyRef":{"DataFrameRef":"2024-01-02","DatedVehicleJourneyRef":"MTA NYCT_JG_D3-Weekday-SDon-100700_B63_662"},"JourneyPatternRef":"MTA_B630027","PublishedLineName":"B63","OperatorRef":"MTA NYCT","OriginRef":"MTA_901601","DestinationName":"BAY RIDGE SHORE RD via 5 AV","OriginAimedDepartureTime":"2024-01-02T16:47:00.000-05:00","SituationRef":[{"SituationSimpleRef":"MTA NYCT_lmm:planned_work:9418"}],"Monitored":true,"VehicleLocation":{"Longitude":-73.978779,"Latitude":40.684562},"Bearing":158.07822,"ProgressRate":"normalProgress","ProgressStatus":"prevTrip","BlockRef":"MTA NYCT_JG_D3-Weekday-SDon_E_JG_41220_B63-661","VehicleRef":"MTA NYCT_254","MonitoredCall":{"AimedArrivalTime":"2024-01-02T17:14:22.000-05:00","ExpectedArrivalTime":"2024-01-02T17:12:55.501-05:00","AimedDepartureTime":"2024-01-02T17:14:22.000-05:00","ExpectedDepartureTime":"2024-01-02T17:12:55.501-05:00","Extensions":{"Distances":{"PresentableDistance":"3.6 miles away","DistanceFromCall":5742.6,"StopsFromCall":15,"CallDistanceAlongRoute":3556.08},"VehicleFeatures":{"StrollerVehicle":false}},"StopPointRef":"MTA_308215","VisitNumber":1,"StopPointName":"5 AV/GARFIELD PL"},"OnwardCalls":{}},"RecordedAtTime":"2024-01-02T16:28:29.000-05:00"},{"MonitoredVehicleJourney":{"LineRef":"MTA NYCT_B63","DirectionRef":"1","FramedVehicleJourneyRef":{"DataFrameRef":"2024-01-02","DatedVehicleJourneyRef":"MTA NYCT_JG_D3-Weekday-SDon-101600_B63_679"},"JourneyPatternRef":"MTA_B630027","PublishedLineName":"B63","OperatorRef":"MTA NYCT","OriginRef":"MTA_901601","DestinationName":"BAY RIDGE SHORE RD via 5 AV","OriginAimedDepartureTime":"2024-01-02T16:56:00.000-05:00","SituationRef":[{"SituationSimpleRef":"MTA NYCT_lmm:planned_work:9418"}],"Monitored":true,"VehicleLocation":{"Longitude":-73.980137,"Latitude":40.676807},"Bearing":56.737186,"ProgressRate":"normalProgress","ProgressStatus":"prevTrip","BlockRef":"MTA NYCT_JG_D3-Weekday-SDon_E_JG_41940_B63-674","VehicleRef":"MTA NYCT_437","MonitoredCall":{"AimedArrivalTime":"2024-01-02T17:23:22.000-05:00","ExpectedArrivalTime":"2024-01-02T17:21:57.626-05:00","AimedDepartureTime":"2024-01-02T17:23:22.000-05:00","ExpectedDepartureTime":"2024-01-02T17:21:57.626-05:00","Extensions":{"Distances":{"PresentableDistance":"4.2 miles away","DistanceFromCall":6773.17,"StopsFromCall":15,"CallDistanceAlongRoute":3556.08},"VehicleFeatures":{"StrollerVehicle":false}},"StopPointRef":"MTA_308215","VisitNumber":1,"StopPointName":"5 AV/GARFIELD PL"},"OnwardCalls":{}},"RecordedAtTime":"2024-01-02T16:28:17.000-05:00"},{"MonitoredVehicleJourney":{"LineRef":"MTA NYCT_B63","DirectionRef":"1","FramedVehicleJourneyRef":{"DataFrameRef":"2024-01-02","DatedVehicleJourneyRef":"MTA NYCT_JG_D3-Weekday-SDon-103500_B63_680"},"JourneyPatternRef":"MTA_B630027","PublishedLineName":"B63","OperatorRef":"MTA NYCT","OriginRef":"MTA_901601","DestinationName":"BAY RIDGE SHORE RD via 5 AV","OriginAimedDepartureTime":"2024-01-02T17:15:00.000-05:00","SituationRef":[{"SituationSimpleRef":"MTA NYCT_lmm:planned_work:9418"}],"Monitored":true,"VehicleLocation":{"Longitude":-74.000615,"Latitude":40.654406},"Bearing":45.10034,"ProgressRate":"normalProgress","ProgressStatus":"prevTrip","BlockRef":"MTA NYCT_JG_D3-Weekday-SDon_E_JG_43380_B63-677","VehicleRef":"MTA NYCT_770","MonitoredCall":{"AimedArrivalTime":"2024-01-02T17:41:22.000-05:00","AimedDepartureTime":"2024-01-02T17:41:22.000-05:00","Extensions":{"Distances":{"PresentableDistance":"6.1 miles away","DistanceFromCall":9809.76,"StopsFromCall":15,"CallDistanceAlongRoute":3556.08},"VehicleFeatures":{"StrollerVehicle":false}},"StopPointRef":"MTA_308215","VisitNumber":1,"StopPointName":"5 AV/GARFIELD PL"},"OnwardCalls":{}},"RecordedAtTime":"2024-01-02T16:28:25.000-05:00"},{"MonitoredVehicleJourney":{"LineRef":"MTA NYCT_B63","DirectionRef":"1","FramedVehicleJourneyRef":{"DataFrameRef":"2024-01-02","DatedVehicleJourneyRef":"MTA NYCT_JG_D3-Weekday-SDon-104500_B63_682"},"JourneyPatternRef":"MTA_B630027","PublishedLineName":"B63","OperatorRef":"MTA NYCT","OriginRef":"MTA_901601","DestinationName":"BAY RIDGE SHORE RD via 5 AV","OriginAimedDepartureTime":"2024-01-02T17:25:00.000-05:00","SituationRef":[{"SituationSimpleRef":"MTA NYCT_lmm:planned_work:9418"}],"Monitored":true,"VehicleLocation":{"Longitude":-74.002757,"Latitude":40.652344},"Bearing":45.0,"ProgressRate":"normalProgress","ProgressStatus":"prevTrip","BlockRef":"MTA NYCT_JG_D3-Weekday-SDon_E_JG_51240_B63-682","VehicleRef":"MTA NYCT_663","MonitoredCall":{"AimedArrivalTime":"2024-01-02T17:51:22.000-05:00","AimedDepartureTime":"2024-01-02T17:51:22.000-05:00","Extensions":{"Distances":{"PresentableDistance":"6.3 miles away","DistanceFromCall":10101.68,"StopsFromCall":15,"CallDistanceAlongRoute":3556.08},"VehicleFeatures":{"StrollerVehicle":false}},"StopPointRef":"MTA_308215","VisitNumber":1,"StopPointName":"5 AV/GARFIELD PL"},"OnwardCalls":{}},"RecordedAtTime":"2024-01-02T16:28:40.000-05:00"}],"ResponseTimestamp":"2024-01-02T16:28:45.691-05:00","ValidUntil":"2024-01-02T16:29:45.691-05:00"}],"SituationExchangeDelivery":[{"Situations":{"PtSituationElement":[{"PublicationWindow":{"StartTime":"2023-09-25T00:00:00.000-04:00","EndTime":"2024-06-29T20:00:00.000-04:00"},"Severity":"undefined","Summary":"Northbound B63 stop on 5th Ave at 36th St has been relocated to 5th Ave at 37th","Description":"Northbound B63 stop on 5th Ave at 36th St has been relocated to 5th Ave at 37th\nWhat happened?\nThe original location has been redesignated as a school bus stop for PS 617K\n\nNote: Real-time tracking on BusTime may be inaccurate in the service change area.","Affects":{"VehicleJourneys":{"AffectedVehicleJourney":[{"LineRef":"MTA NYCT_B63","DirectionRef":"1"},{"LineRef":"MTA NYCT_B63","DirectionRef":"0"}]}},"CreationTime":"2023-03-31T09:20:21.000-04:00","SituationNumber":"MTA NYCT_lmm:planned_work:9418"}]}}]}}}"#;

        assert_eq!(
            super::parse_response(&raw_json).expect("parse_response"),
            vec![
                time::macros::datetime!(2024-01-02 16:38:08.174 -5),
                time::macros::datetime!(2024-01-02 16:46:57.163 -5),
                time::macros::datetime!(2024-01-02 17:03:57.626 -5),
                time::macros::datetime!(2024-01-02 17:12:55.501 -5),
                time::macros::datetime!(2024-01-02 17:21:57.626 -5),
                time::macros::datetime!(2024-01-02 17:41:22.000 -5),
                time::macros::datetime!(2024-01-02 17:51:22.000 -5),
            ]);
    }
}
