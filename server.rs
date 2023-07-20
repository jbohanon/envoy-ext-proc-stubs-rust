use std;
use std::net::ToSocketAddrs;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{transport::Server, Response, Status};
use envoy_ext_proc_stubs::envoy::service::ext_proc::v3;
use envoy_ext_proc_stubs::envoy::r#type::v3::{HttpStatus, StatusCode};
use envoy_ext_proc_stubs::envoy::config::core::v3::{HeaderValueOption, HeaderValue};


#[derive(Debug)]
struct ExternalProcessorImpl;
#[tonic::async_trait]
trait ExternalProcessor: Send + Sync + 'static {
  type ProcessStream: StreamExt<Item = Result<v3::ProcessingResponse, Status>>
      + Send
      + 'static;
  async fn process(
      &self,
      request: tonic::Request<tonic::Streaming<v3::ProcessingRequest>>,
  ) -> Result<tonic::Response<Self::ProcessStream>, tonic::Status>;
}


#[tonic::async_trait]
impl v3::external_processor_server::ExternalProcessor for ExternalProcessorImpl {

  type ProcessStream = ReceiverStream<Result<v3::ProcessingResponse, Status>>;


  async fn process(
    &self,
    request: tonic::Request<tonic::Streaming<v3::ProcessingRequest>>,
  ) -> Result<tonic::Response<Self::ProcessStream>, tonic::Status> {
    let mut stream: tonic::Streaming<v3::ProcessingRequest> = request.into_inner();
    let (tx, rx) = tokio::sync::mpsc::channel(32);
    println!("process called");

    tokio::spawn(async move {
        while let Some(request) = stream.next().await {
            match request {
                Ok(req) => {
                    // Process the request and return a response.
                    let response: v3::ProcessingResponse = process_request(req);
                    tx.send(Ok(response)).await.unwrap();
                },
                Err(e) => {
                  match e.code() {
                    tonic::Code::Unknown => {
                      println!("Unknown error code, means stream was ended by client");
                      return;
                    },
                    _ => {},
                  }
                  match tx.send(Err(Status::internal("Internal error"))).await {
                    Ok(_) => {},
                    Err(e) => {
                      println!("Error sending error: ({:?}). say that 5 times fast", e);
                    }
                  }
                  // tx.send(Err(Status::internal("Internal error"))).await.unwrap();
                },
            }
        }
    });

    Ok(Response::new(ReceiverStream::new(rx)))
  }
}

// process the incoming processing request
fn process_request(request: v3::ProcessingRequest) -> v3::ProcessingResponse {

  let mut response = v3::ProcessingResponse {
    dynamic_metadata: None,
    mode_override: None,
    response: None,
  };
  // TODO: Handle non-existent I guess
  match request.request.unwrap() {
    v3::processing_request::Request::RequestHeaders(headers) => {
      response.response = Some(v3::processing_response::Response::RequestHeaders(v3::HeadersResponse {
        ..Default::default()
      }));
      println!("Request Headers: {:?}", headers.headers)
    },
    v3::processing_request::Request::RequestBody(body) => {
      println!("Request Body: {:?}", body.body);
      match process_body(body) {
        BodyResponse::Body(b) => {
          response.response = Some(v3::processing_response::Response::RequestBody(b));
        },
        BodyResponse::Immediate(i) => {
          response.response = Some(v3::processing_response::Response::ImmediateResponse(i));
        }
      }
    },
    v3::processing_request::Request::RequestTrailers(trailers) => {
      response.response = Some(v3::processing_response::Response::RequestTrailers(v3::TrailersResponse {
        ..Default::default()
      }));
      println!("Request Trailers: {:?}", trailers.trailers)
    },
    v3::processing_request::Request::ResponseHeaders(headers) => {
      response.response = Some(v3::processing_response::Response::ResponseHeaders(v3::HeadersResponse {
        ..Default::default()
      }));
      println!("Response Headers: {:?}", headers.headers)
    },
    v3::processing_request::Request::ResponseBody(body) => {
      println!("Response Body: {:?}", body.body);
      match process_body(body) {
        BodyResponse::Body(b) => {
          response.response = Some(v3::processing_response::Response::RequestBody(b));
        },
        BodyResponse::Immediate(i) => {
          response.response = Some(v3::processing_response::Response::ImmediateResponse(i));
        }
      }
    },
    v3::processing_request::Request::ResponseTrailers(trailers) => {
      response.response = Some(v3::processing_response::Response::ResponseTrailers(v3::TrailersResponse {
        ..Default::default()
      }));
      println!("Response Trailers: {:?}", trailers.trailers)
    },
  }

  response
}

enum BodyResponse {
  Body(v3::BodyResponse),
  Immediate(v3::ImmediateResponse),
}

fn process_body(body: v3::HttpBody) -> BodyResponse {
  match serde_json::from_slice::<serde_json::Value>(body.body.as_ref()) {
    Ok(v) => {
      return BodyResponse::Body(v3::BodyResponse {
        response: Some(v3::CommonResponse{
          status: v3::common_response::ResponseStatus::ContinueAndReplace as i32,
          body_mutation: Some(v3::BodyMutation{
            mutation: Some(v3::body_mutation::Mutation::Body(prost::bytes::Bytes::from("{}")))
          }),
          header_mutation: Some(v3::HeaderMutation{
            set_headers: vec![HeaderValueOption{
              header: Some(HeaderValue{
                key: prost::alloc::string::String::from("content-length"),
                value: prost::alloc::string::String::from("2"),
              }),
              ..Default::default()
            }],
            ..Default::default()
          }),
          ..Default::default()
        })
          // ..Default::default()
      });
    },
    Err(e) => {
      println!("Error parsing body: {:?}", e);
      return BodyResponse::Immediate(v3::ImmediateResponse{
        body: prost::alloc::string::String::from("invalid json body"),
        details: prost::alloc::string::String::from("invalid json body"),
        grpc_status: None,
        headers: None,
        status: Some(HttpStatus {
          code: StatusCode::PreconditionFailed as i32,
        })
      })
    }
  }
}

struct FilterState {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let server = ExternalProcessorImpl {};
  Server::builder()
      .add_service(v3::external_processor_server::ExternalProcessorServer::new(server))
      .serve("0.0.0.0:9090".to_socket_addrs().unwrap().next().unwrap())
      .await
      .unwrap();

  Ok(())
}