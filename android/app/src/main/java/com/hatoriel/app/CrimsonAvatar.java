package com.hatoriel.app;
import android.content.Context;
import android.graphics.*;
import android.view.View;
import android.os.SystemClock;

final class CrimsonAvatar extends View {
    static final int RED=0xffee3434;
    enum State {
        IDLE("Очікування"), DICTATION("Системне диктування"), THINKING("Обробка запиту"), RESPONDING("Генерація відповіді"), SPEAKING("Озвучування");
        final String label;
        State(String label){this.label=label;}
        static State select(boolean external, boolean busy, boolean streaming, boolean speaking){
            if(external)return DICTATION;
            if(busy)return streaming?RESPONDING:THINKING;
            return speaking?SPEAKING:IDLE;
        }
    }
    private final Paint p=new Paint(Paint.ANTI_ALIAS_FLAG);
    private State state=State.IDLE;
    private boolean motion=true;
    private final float[] from=new float[4],target=new float[4],expression=new float[4];
    private long transitionStart;
    private boolean canAnimate(){return motion&&android.animation.ValueAnimator.areAnimatorsEnabled();}
    private void sample(long now){float u=canAnimate()?Math.max(0f,Math.min(1f,(now-transitionStart)/350f)):1f;u=u*u*(3f-2f*u);for(int i=0;i<4;i++)expression[i]=from[i]+(target[i]-from[i])*u;}
    void setMood(String mood,int intensity){int index=switch(mood){case "Joy"->0;case "Concern"->1;case "Surprise"->2;case "Curiosity"->3;default->-1;};float strength=Math.max(0,Math.min(100,intensity))/100f;boolean changed=false;for(int i=0;i<4;i++)if(target[i]!=(i==index?strength:0f))changed=true;if(!changed)return;long now=SystemClock.elapsedRealtime();sample(now);for(int i=0;i<4;i++){from[i]=expression[i];target[i]=i==index?strength:0f;}transitionStart=now;invalidate();}
    CrimsonAvatar(Context context){super(context);setContentDescription("Hatoriel · "+state.label);}
    void setMotion(boolean enabled){if(motion!=enabled){motion=enabled;invalidate();}}
    void setState(State next){if(state!=next){state=next;setContentDescription("Hatoriel · "+state.label);invalidate();}}
    @Override protected void onDraw(Canvas canvas){
        super.onDraw(canvas);
        boolean animated=canAnimate();sample(SystemClock.elapsedRealtime());float joy=expression[0],concern=expression[1],surprise=expression[2],curiosity=expression[3];
        float t=animated?(SystemClock.elapsedRealtime()%60000)/1000f:0f;
        float x=getWidth()/2f,y=getHeight()/2f;
        float speed=state==State.SPEAKING?6:state==State.RESPONDING?5:state==State.THINKING?3:1.8f;
        float r=Math.min(x,y)*.51f*(1f+.055f*joy-.045f*concern+.075f*surprise)*(1f+(state==State.SPEAKING?.065f:.035f)*(float)Math.sin(t*speed));
        float density=getResources().getDisplayMetrics().density;
        p.setStyle(Paint.Style.STROKE);p.setColor(RED);
        p.setAlpha(Math.min(255,24+Math.round(42*Math.min(1f,joy+.45f*concern+surprise+.65f*curiosity))));p.setStrokeWidth(7*density);canvas.drawCircle(x,y,r,p);
        p.setAlpha(255);p.setStrokeWidth(2*density);canvas.drawCircle(x,y,r,p);
        for(int i=0;i<4;i++){double a=i*Math.PI/2;float dx=(float)Math.cos(a),dy=(float)Math.sin(a);canvas.drawLine(x+dx*r*.68f,y+dy*r*.68f,x+dx*r*1.42f,y+dy*r*1.42f,p);}
        p.setStyle(Paint.Style.FILL);canvas.drawCircle(x,y,Math.max(1.5f*density,r*(.055f+.035f*surprise)),p);
        p.setStyle(Paint.Style.STROKE);p.setStrokeWidth(1.4f*density);
        if(joy>.001f){p.setAlpha(Math.round(204*joy));canvas.drawCircle(x,y,r*.80f,p);}
        if(concern>.001f){p.setAlpha(Math.round(191*concern));canvas.drawCircle(x,y,r*.55f,p);}
        if(surprise>.001f){p.setAlpha(Math.round(255*surprise));float e=r*1.16f;for(int i=0;i<4;i++)canvas.drawArc(x-e,y-e,x+e,y+e,18+i*90,26,false,p);}
        if(curiosity>.001f){p.setAlpha(Math.round(217*curiosity));float e=r*.77f;for(int i=0;i<2;i++)canvas.drawArc(x-e,y-e,x+e,y+e,t*46+i*180,54,false,p);}
        p.setStrokeWidth(1.5f*density);p.setAlpha(180);
        if(state==State.THINKING){float outer=r*1.65f;for(int i=0;i<3;i++)canvas.drawArc(x-outer,y-outer,x+outer,y+outer,t*85+i*120,60,false,p);}
        else if(state==State.DICTATION){p.setAlpha(115);canvas.drawCircle(x,y,r*1.54f,p);}
        else if(state==State.RESPONDING){p.setAlpha(100);canvas.drawCircle(x,y,r*(1.48f+.08f*(float)Math.sin(t*4)),p);}
        else if(state==State.SPEAKING){for(int i=-3;i<=3;i++){float bx=x+i*r*.28f,by=y+r*1.55f,h=r*(.12f+.17f*(float)Math.abs(Math.sin(t*6+i*.9f)));canvas.drawLine(bx,by-h,bx,by+h,p);}}
        if(animated&&isAttachedToWindow()&&isShown()&&getWindowVisibility()==VISIBLE)postInvalidateDelayed(state==State.IDLE&&SystemClock.elapsedRealtime()-transitionStart>=350?100:33);
    }
    @Override protected void onVisibilityChanged(View changed,int visibility){super.onVisibilityChanged(changed,visibility);if(visibility==VISIBLE)invalidate();}
    @Override protected void onWindowVisibilityChanged(int visibility){super.onWindowVisibilityChanged(visibility);if(visibility==VISIBLE)invalidate();}
}
