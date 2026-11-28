package com.hatoriel.app;
import android.content.SharedPreferences;
import org.json.*;
import java.util.Locale;

final class MobileMemory {
    static final String[] CATEGORIES={"General","Interests","Preferences","Skills","Goals","Projects","Relationships","Other"};
    private final SharedPreferences prefs;
    MobileMemory(SharedPreferences prefs){this.prefs=prefs;}
    JSONArray records(){try{return new JSONArray(prefs.getString("approved_memories","[]"));}catch(JSONException e){return new JSONArray();}}
    JSONObject profile(){try{return new JSONObject(prefs.getString("memory_profile","{}"));}catch(JSONException e){return new JSONObject();}}
    boolean enabled(){return prefs.getBoolean("memory_context",false);}
    boolean suggestions(){return prefs.getBoolean("memory_suggestions",true);}
    void setEnabled(boolean value){prefs.edit().putBoolean("memory_context",value).apply();}
    void setSuggestions(boolean value){prefs.edit().putBoolean("memory_suggestions",value).apply();}
    private static String normalize(String s){return s.trim().replaceAll("\\s+"," ").toLowerCase(Locale.ROOT);}
    boolean contains(String content){JSONArray rows=records();for(int i=0;i<rows.length();i++){JSONObject m=rows.optJSONObject(i);if(m!=null&&normalize(m.optString("content")).equals(normalize(content)))return true;}return false;}
    void save(String content,String category,int importance)throws JSONException{
        content=content.trim();
        if(content.isEmpty()||content.codePointCount(0,content.length())>500)throw new IllegalArgumentException("Спогад: від 1 до 500 символів");
        if(contains(content))throw new IllegalArgumentException("Такий спогад уже існує");
        JSONArray rows=records();if(rows.length()>=100)throw new IllegalArgumentException("Ліміт 100 спогадів");
        long id=Math.max(System.currentTimeMillis(),prefs.getLong("memory_last_id",0)+1);
        rows.put(new JSONObject().put("id",id).put("content",content).put("category",category).put("importance",Math.max(0,Math.min(100,importance))));
        if(!prefs.edit().putString("approved_memories",rows.toString()).putLong("memory_last_id",id).commit())throw new IllegalStateException("Не вдалося зберегти спогад");
    }
    void remove(long id){JSONArray rows=records();for(int i=0;i<rows.length();i++){JSONObject m=rows.optJSONObject(i);if(m!=null&&m.optLong("id")==id){rows.remove(i);break;}}prefs.edit().putString("approved_memories",rows.toString()).apply();}
    void saveProfile(String interests,String communication,String goals)throws JSONException{
        JSONObject p=new JSONObject();p.put("interests",limit(interests));p.put("communication",limit(communication));p.put("goals",limit(goals));
        prefs.edit().putString("memory_profile",p.toString()).apply();
    }
    private static String limit(String s){return s.codePointCount(0,s.length())<=1000?s:s.substring(0,s.offsetByCodePoints(0,1000));}
    JSONObject options()throws JSONException{
        return new JSONObject(prefs.getString("sync_options","{\"mode\":\"Companion\",\"traits\":[90,80,90,90,90,80,70],\"dynamic_style\":true,\"emotion_enabled\":false,\"mood\":{}}"));
    }
    void savePersonality(String mode,int[] traits,boolean dynamic,boolean emotional)throws JSONException{
        if(!java.util.Arrays.asList("Companion","Assistant","Developer","Researcher").contains(mode)||traits.length!=7)throw new IllegalArgumentException("Invalid personality");
        JSONArray values=new JSONArray();for(int t:traits){if(t<0||t>100)throw new IllegalArgumentException("Invalid trait");values.put(t);}
        JSONObject opts=new JSONObject().put("mode",mode).put("traits",values).put("dynamic_style",dynamic).put("emotion_enabled",emotional).put("mood",new JSONObject());
        if(!prefs.edit().putString("sync_options",opts.toString()).commit())throw new IllegalStateException("Не вдалося зберегти особистість");
    }
    JSONObject snapshot()throws JSONException{
        return new JSONObject().put("version",1).put("records",new JSONArray(prefs.getString("approved_memories","[]"))).put("profile",new JSONObject(prefs.getString("memory_profile","{}"))).put("options",options());
    }
    void applySync(JSONObject plan,boolean profile,boolean personality)throws JSONException{
        JSONArray rows=new JSONArray(prefs.getString("approved_memories","[]")), additions=plan.getJSONArray("additions");
        if(rows.length()+additions.length()>100)throw new IllegalArgumentException("Ліміт 100 спогадів");
        long id=Math.max(System.currentTimeMillis(),prefs.getLong("memory_last_id",0)+1);
        for(int n=0;n<additions.length();n++){
            JSONObject row=new JSONObject(additions.getJSONObject(n).toString());
            row.put("id",id++);rows.put(row);
        }
        SharedPreferences.Editor edit=prefs.edit().putString("approved_memories",rows.toString()).putLong("memory_last_id",id).putBoolean("memory_context",false);
        if(profile)edit.putString("memory_profile",plan.getJSONObject("profile").toString());
        if(personality)edit.putString("sync_options",plan.getJSONObject("options").toString());
        if(!edit.commit())throw new IllegalStateException("Не вдалося зберегти імпорт");
    }
    String context(JSONArray history)throws JSONException{
        JSONObject data=new JSONObject().put("history",history);
        data.put("options",options());
        data.put("memories",enabled()?records():new JSONArray());
        data.put("profile",enabled()?profile():new JSONObject());
        return data.toString();
    }
}
