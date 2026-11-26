#include "llama.h"
#include <algorithm>
#include <chrono>
#include <cstdio>
#include <cstring>
#include <memory>
#include <mutex>
#include <vector>
#include <string>
#include <cstdint>
using callback_t = int (*)(const uint8_t*,size_t,void*);
namespace {
std::mutex gate;
struct Session {
 callback_t callback;void* opaque;
 std::chrono::steady_clock::time_point start=std::chrono::steady_clock::now();
 bool stopped(){return std::chrono::steady_clock::now()-start>std::chrono::minutes(5)||!callback(nullptr,0,opaque);}
};
int fail(char* error,size_t size,const char* text){if(error&&size)std::snprintf(error,size,"%s",text);return 1;}
}
extern "C" int hatoriel_local_generate(const char* path,const char* const* roles,const char* const* contents,size_t count,int32_t context,int32_t threads,callback_t cb,void* opaque,char* error,size_t error_size){
 try {
  std::unique_lock<std::mutex> lock(gate,std::try_to_lock);
  if(!lock.owns_lock())return fail(error,error_size,"Local runtime busy");
  if(!path||!roles||!contents||!cb||count==0||count>33||context<1024||context>4096||threads<1||threads>6)return fail(error,error_size,"Invalid local request");
  Session session{cb,opaque};
  static std::once_flag init;std::call_once(init,[]{llama_backend_init();});
  auto mp=llama_model_default_params();mp.n_gpu_layers=0;
  mp.progress_callback=[](float,void* p){return !static_cast<Session*>(p)->stopped();};
  mp.progress_callback_user_data=&session;
  std::unique_ptr<llama_model,decltype(&llama_model_free)> model(llama_model_load_from_file(path,mp),llama_model_free);
  if(!model||session.stopped())return fail(error,error_size,"Model load failed or cancelled");
  if(llama_model_has_encoder(model.get())||!llama_model_has_decoder(model.get()))return fail(error,error_size,"Only decoder-only chat models supported");
  const auto* tmpl=llama_model_chat_template(model.get(),nullptr);
  if(!tmpl)return fail(error,error_size,"Model has no supported chat template");
  std::vector<llama_chat_message> messages;size_t total=0;
  for(size_t i=0;i<count;i++){if(!roles[i]||!contents[i])return fail(error,error_size,"Invalid messages");total+=std::strlen(contents[i]);if(total>262144)return fail(error,error_size,"Prompt too large");messages.push_back({roles[i],contents[i]});}
  std::vector<char> prompt(16384);
  int n=llama_chat_apply_template(tmpl,messages.data(),messages.size(),true,prompt.data(),prompt.size());
  if(n<=0||n>262144)return fail(error,error_size,"Unsupported chat template");
  if(n>=int(prompt.size())){prompt.resize(n+1);n=llama_chat_apply_template(tmpl,messages.data(),messages.size(),true,prompt.data(),prompt.size());}
  if(n<=0||n>=int(prompt.size()))return fail(error,error_size,"Chat template failed");
  const auto* vocab=llama_model_get_vocab(model.get());
  int tokens_needed=-llama_tokenize(vocab,prompt.data(),n,nullptr,0,true,true);
  if(tokens_needed<=0||tokens_needed>=context-32)return fail(error,error_size,"Context too long; start a shorter chat or increase context");
  std::vector<llama_token> tokens(tokens_needed);
  if(llama_tokenize(vocab,prompt.data(),n,tokens.data(),tokens.size(),true,true)!=tokens_needed)return fail(error,error_size,"Tokenization failed");
  auto cp=llama_context_default_params();cp.n_ctx=context;cp.n_batch=context;cp.n_ubatch=128;cp.n_threads=threads;cp.n_threads_batch=threads;
  cp.abort_callback=[](void* p){return static_cast<Session*>(p)->stopped();};cp.abort_callback_data=&session;
  std::unique_ptr<llama_context,decltype(&llama_free)> ctx(llama_init_from_model(model.get(),cp),llama_free);
  if(!ctx)return fail(error,error_size,"Context allocation failed");
  std::unique_ptr<llama_sampler,decltype(&llama_sampler_free)> sampler(llama_sampler_init_greedy(),llama_sampler_free);
  if(!sampler)return fail(error,error_size,"Sampler allocation failed");
  auto batch=llama_batch_get_one(tokens.data(),tokens.size());
  int limit=std::min(512,static_cast<int>(llama_n_ctx(ctx.get()))-tokens_needed);
  if(limit<=0)return fail(error,error_size,"Context exhausted");
  size_t output=0;llama_token token=0;
  for(int i=0;i<limit;i++){
   if(session.stopped())return fail(error,error_size,"Cancelled or time limit");
   if(llama_decode(ctx.get(),batch)!=0)return fail(error,error_size,"Decode failed or cancelled");
   token=llama_sampler_sample(sampler.get(),ctx.get(),-1);
   if(llama_vocab_is_eog(vocab,token))break;
   char piece[4096];int len=llama_token_to_piece(vocab,token,piece,sizeof(piece),0,true);
   if(len<0||output+size_t(len)>262144)return fail(error,error_size,"Output exceeds limit");
   if(len>0){output+=len;if(!cb(reinterpret_cast<const uint8_t*>(piece),len,opaque))return fail(error,error_size,"Cancelled");}
   batch=llama_batch_get_one(&token,1);
  }
  return output?0:fail(error,error_size,"Empty response");
 }catch(const std::exception& e){return fail(error,error_size,e.what());}catch(...){return fail(error,error_size,"Native runtime error");}
}
