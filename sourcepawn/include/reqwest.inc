#if defined _reqwest_included
  #endinput
#endif
#define _reqwest_included

enum ReqwestMethod
{
	ReqwestMethod_Get = 1,
	ReqwestMethod_Head = 2,
	ReqwestMethod_Post = 3,
	ReqwestMethod_Put = 4,
	ReqwestMethod_Delete = 5,
	ReqwestMethod_Connect = 6,
	ReqwestMethod_Options = 7,
	ReqwestMethod_Trace = 8,
	ReqwestMethod_Patch = 9,
}

methodmap ReqwestClient < Handle
{
	public native ReqwestClient(const char[] proxy_scheme = NULL_STRING);

	public native ReqwestRequest Request(ReqwestMethod method, const char[] url);
}

typedef ReqwestRequest_ResponseCallback = function void (ReqwestRequest self, ReqwestResponse resp, const char[] err_msg, any data);

typedef ReqwestResponse_RecvCallback = function void (ReqwestResponse self, const char[] content, int content_len, const char[] err_msg, any data);

methodmap ReqwestRequest < Handle
{
	public native ReqwestRequest SetHeader(const char[] header, const char[] value);

	public native ReqwestRequest SetBody(const char[] data, int data_len = 0);

	public native ReqwestRequest SetBodyWithFile(const char[] filepath);

	public native ReqwestRequest SetTimeout(int timeout_ms);

	property int SpeedLimit {
		public native get();

		public native set(int bytesps);
	}

	public native void SendAsync(ReqwestRequest_ResponseCallback callback, any data = 0);

	public native ReqwestResponse Send(char[] err_msg, int errlen);
}

methodmap ReqwestResponse < Handle
{
	property int HeadersLength {
		public native get();
	}

	public native int GetHeaderKey(int n, char[] key, int size);

	public native int GetHeaderValue(int n, char[] key, int size);

	public native int GetHeaderValueOfKey(const char[] key, char[] value, int size);

	property int StatusCode {
		public native get();
	}

	property int ContentLength {
		public native get();
	}

	property int SpeedLimit {
		public native get();

		public native set(int bytesps);
	}

	public native void RecvAsync(ReqwestResponse_RecvCallback callback, any data = 0, int min_size = 0);

	public native void RecvWholeAsync(ReqwestResponse_RecvCallback callback, any data = 0);

	public native int Recv(char[] content, int content_len, char[] err_msg, int errlen, int min_size = 0);

	public native int RecvWhole(char[] content, int content_len, char[] err_msg, int errlen);
}

native void Reqwest_URLEncode(char[] output, int maxlength, const char[] input, any ...);
native void Reqwest_URLDecode(char[] output, int maxlength, const char[] input, any ...);

public Extension __ext_reqwest = 
{
	name = "Reqwest",
	file = "reqwest.ext",
#if defined AUTOLOAD_EXTENSIONS
	autoload = 1,
#else
	autoload = 0,
#endif
#if defined REQUIRE_EXTENSIONS
	required = 1,
#else
	required = 0,
#endif
};

#if !defined REQUIRE_EXTENSIONS
public void __ext_reqwest_SetNTVOptional()
{
	MarkNativeAsOptional("ReqwestClient.ReqwestClient");
	MarkNativeAsOptional("ReqwestClient.Request");

	MarkNativeAsOptional("ReqwestRequest.SetHeader");
	MarkNativeAsOptional("ReqwestRequest.SetBody");
	MarkNativeAsOptional("ReqwestRequest.SetBodyWithFile");
	MarkNativeAsOptional("ReqwestRequest.SetTimeout");
	MarkNativeAsOptional("ReqwestRequest.SpeedLimit.get");
	MarkNativeAsOptional("ReqwestRequest.SpeedLimit.set");
	MarkNativeAsOptional("ReqwestRequest.SendAsync");
	MarkNativeAsOptional("ReqwestRequest.Send");
	
	MarkNativeAsOptional("ReqwestResponse.HeadersLength.get");
	MarkNativeAsOptional("ReqwestResponse.GetHeaderKey");
	MarkNativeAsOptional("ReqwestResponse.GetHeaderValue");
	MarkNativeAsOptional("ReqwestResponse.GetHeaderValueOfKey");
	MarkNativeAsOptional("ReqwestResponse.StatusCode.get");
	MarkNativeAsOptional("ReqwestResponse.Length.get");
	MarkNativeAsOptional("ReqwestResponse.SpeedLimit.get");
	MarkNativeAsOptional("ReqwestResponse.SpeedLimit.set");
	MarkNativeAsOptional("ReqwestResponse.RecvAsync");
	MarkNativeAsOptional("ReqwestResponse.RecvWholeAsync");
	MarkNativeAsOptional("ReqwestResponse.Recv");
	MarkNativeAsOptional("ReqwestResponse.RecvWhole");

	MarkNativeAsOptional("Reqwest_URLEncode");
	MarkNativeAsOptional("Reqwest_URLDecode");
}
#endif
