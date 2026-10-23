# Đặc tả Kỹ thuật: Mở rộng Đa Giao thức — GraphQL, WebSocket, SSE & gRPC (Protocols Specification)

## 1. Bối cảnh
API hiện đại không còn chỉ là các endpoint REST tĩnh. Các kiến trúc phần mềm ngày nay phổ biến các giao thức:
- **GraphQL:** Cung cấp khả năng truy vấn trường linh hoạt theo yêu cầu client.
- **WebSocket:** Giao tiếp hai chiều thời gian thực (chat, notifications, sàn tài chính, dashboard).
- **Server-Sent Events (SSE):** Truyền dữ liệu dạng dòng (streaming tokens) đặc biệt phổ biến trong kỷ nguyên AI / Large Language Models (OpenAI, Gemini, Anthropic API).
- **gRPC:** Giao tiếp microservice nội bộ với hiệu năng cao dựa trên HTTP/2 và Protobuf.

Tài liệu này xác định cách thiết kế và mở rộng kiến trúc Zest để hỗ trợ đầy đủ các giao thức trên.

---

## 2. GraphQL Client

### 2.1 Cấu trúc Dữ liệu Model
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLPayload {
    pub query: String,
    pub variables: String, // Raw JSON string
    pub operation_name: Option<String>,
}
```

### 2.2 Tính năng Cốt lõi:
1. **Chia 2 Pane Soạn Thảo (Dual-pane editor):**
   - **Query Pane (trên/trái):** Soạn thảo `query` hoặc `mutation`.
   - **Variables Pane (dưới/phải):** Soạn thảo tham số JSON cho query.
2. **Schema Introspection:**
   - Khi người dùng bấm nút **"Fetch Schema"**, gửi truy vấn introspection chuẩn GraphQL:
     ```graphql
     query IntrospectionQuery {
       __schema {
         queryType { name }
         mutationType { name }
         types { ...FullType }
       }
     }
     ```
   - Lưu trữ schema AST vào bộ nhớ đệm (cache) của request.
3. **Gợi ý Trường & Tự Động Hoàn Thành (Autocomplete):**
   - Đọc schema AST để gợi ý field name, subfield, argument types khi người dùng gõ phím.

---

## 3. WebSocket Client

### 3.1 Luồng Kết nối & Quản lý State
WebSocket đòi hỏi một kết nối bền vững (persistent connection), khác biệt hoàn toàn với HTTP Request/Response ngắn hạn.

```rust
pub enum WebSocketStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed(String),
}

pub struct WebSocketMessage {
    pub id: String,
    pub timestamp: i64,
    pub is_inbound: bool, // true: server -> client, false: client -> server
    pub message_type: WsMessageType, // Text, Binary, Ping, Pong
    pub payload: Vec<u8>,
}
```

### 3.2 Tầng Network (Tokio `tokio-tungstenite`)
- Tạo background task duy trì connection loop.
- Gửi sự kiện message mới qua channel `tokio::sync::mpsc::unbounded_channel()` về GPUI UI thread.
- Cho phép người dùng gửi tin nhắn nhanh dạng Text, JSON, hoặc Ping heartbeat.

---

## 4. Server-Sent Events (SSE) & Streaming Responses

### 4.1 Ứng dụng Thực Tế
Khi kiểm thử các endpoint AI streaming (như `POST https://api.openai.com/v1/chat/completions` với `{"stream": true}`), response trả về theo từng dòng `data: {"choices": [{"delta": {"content": "Hello"}}]}\n\n`.

### 4.2 Thiết kế Bộ đệm Dòng (Streaming Buffer)
- `ReqwestExecutor` sử dụng `response.bytes_stream()`.
- Mỗi khi nhận một packet `Bytes`, decode chuỗi UTF-8 và bắn sự kiện `HttpSendEvent::StreamChunk(String)` về GPUI.
- UI cập nhật liên tục nội dung vừa sinh ra, tự động cuộn xuống cuối (Auto-scroll), mang lại trải nghiệm như đang dùng ứng dụng chat AI.

---

## 5. gRPC Client (Lộ trình Phase 3+)

### 5.1 Kiến trúc Thực thi
- Sử dụng crate **`tonic`** (gRPC trên nền Rust và Tokio).
- Hỗ trợ nạp file `.proto` hoặc **gRPC Server Reflection Protocol** (`grpc.reflection.v1alpha.ServerReflection`).
- Tự động sinh danh sách Service và Phương thức (RPC Method).
- Người dùng nhập dữ liệu dưới dạng JSON thân thiện, tầng core tự động mã hóa sang Protobuf binary bytes để gửi đi qua HTTP/2.
