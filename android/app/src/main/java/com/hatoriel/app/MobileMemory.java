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
    String context(JSONArray history)throws JSONException{
        JSONObject data=new JSONObject().put("history",history);
        data.put("memories",enabled()?records():new JSONArray());
        data.put("profile",enabled()?profile():new JSONObject());
        return data.toString();
    }
}
