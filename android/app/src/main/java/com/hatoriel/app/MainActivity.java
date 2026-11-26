package com.hatoriel.app;
import android.app.Activity;
import android.os.Bundle;
import android.widget.*;
import android.graphics.Color;
public final class MainActivity extends Activity {
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        LinearLayout root=new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);root.setPadding(32,48,32,32);
        root.setBackgroundColor(Color.rgb(11,17,25));
        TextView title=new TextView(this);title.setTextSize(26);title.setTextColor(Color.rgb(92,228,213));title.setText("HATORIEL");
        root.addView(title);
        TextView status=new TextView(this);root.addView(status);
        EditText input=new EditText(this);input.setHint("Тестове повідомлення");root.addView(input);
        Button test=new Button(this);test.setText("Перевірити Rust Core");root.addView(test);
        ScrollView scroll=new ScrollView(this);TextView output=new TextView(this);scroll.addView(output);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
        try {status.setText("JNI Core "+NativeCore.version()+" · діагностика, не чат");}
        catch(LinkageError e){status.setText("JNI unavailable: "+e.getMessage());test.setEnabled(false);}
        test.setOnClickListener(v->{try{output.setText(NativeCore.prepare(input.getText().toString()));}catch(LinkageError|RuntimeException e){output.setText(e.toString());}});
        setContentView(root);
    }
}
