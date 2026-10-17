package org.solosoul.rf312transport;

import android.os.Process;
import android.os.SystemClock;
import android.system.Os;
import android.system.StructStat;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.Base64;

/** Owned Observer-file transport; no target/UI, network or subprocess APIs. */
public final class OwnedUi {
    static final int LIMIT=4194304, ERROR_LIMIT=65536, PAYLOAD_LIMIT=16384;
    static File base;
    static final class Refusal extends Exception {
        final int code; final byte[] out,err;
        Refusal(int code) { this(code,new byte[0]); }
        Refusal(int code,byte[] out) { this(code,out,new byte[0]); }
        Refusal(int code,byte[] out,byte[] err) { this.code=code;this.out=out;this.err=err; }
    }
    static final class Reply {
        int code;byte[] out=new byte[0],err=new byte[0];
    }
    static long seconds() { return SystemClock.elapsedRealtimeNanos()/1000000000L; }
    static void require(boolean value,int code)throws Refusal { if(!value)throw new Refusal(code); }
    static File file(String name)throws Refusal {
        require(name.matches("ready|error|stopped|response-[0-9]{3}\\.json|frame-[0-9]{3}\\.png"),64);
        return new File(base,name);
    }
    static byte[] stableRead(String name)throws Refusal,IOException {
        File value=file(name);ByteArrayOutputStream out=new ByteArrayOutputStream();
        long before=value.length();require(value.isFile()&&before>0&&before<=LIMIT,65);
        SystemClock.sleep(25);
        require(value.length()==before,65);
        try(InputStream in=new FileInputStream(value)) {
            byte[] buffer=new byte[8192];int got;
            while((got=in.read(buffer))!=-1) {
                if(out.size()+got>LIMIT)throw new Refusal(65,out.toByteArray());
                out.write(buffer,0,got);
            }
        } catch(IOException error) { throw new Refusal(65,out.toByteArray(),error.toString().getBytes(StandardCharsets.UTF_8)); }
        if(value.length()!=before||out.size()!=before)throw new Refusal(65,out.toByteArray());
        return out.toByteArray();
    }
    static byte[] exchange(String name,String payload)throws Refusal,IOException {
        require(name.matches("request-[0-9]{3}\\.json"),64);
        int index=Integer.parseInt(name.substring(8,11));require(index>=1&&index<=512,64);
        String id=name.substring(8,11);File request=new File(base,name);
        File response=new File(base,"response-"+id+".json");File temporary=new File(base,"request-"+id+".tmp");
        // Preserve the four ordered pre-publication guards.
        require(!request.exists(),64);require(!response.exists(),64);require(!temporary.exists(),64);
        require(!new File(base,"error").exists(),70);
        require(payload.matches("[A-Za-z0-9+/=]+")&&payload.length()<=21848,64);
        byte[] bytes;
        try { bytes=Base64.getDecoder().decode(payload); }
        catch(IllegalArgumentException invalid) { throw new Refusal(65); }
        require(bytes.length>0&&bytes.length<=PAYLOAD_LIMIT,65);
        try(OutputStream out=Files.newOutputStream(temporary.toPath(),StandardOpenOption.CREATE_NEW,StandardOpenOption.WRITE)) { out.write(bytes); }
        Files.move(temporary.toPath(),request.toPath(),StandardCopyOption.ATOMIC_MOVE);
        long deadline=seconds()+45;
        while(true) {
            require(seconds()<deadline,71);
            if(response.exists())break;
            require(!new File(base,"error").exists(),70);
            require(!new File(base,"stopped").exists(),70);
            SystemClock.sleep(25);
        }
        require(!new File(base,"error").exists(),70);
        return stableRead(response.getName());
    }
    static String line(InputStream input)throws IOException {
        ByteArrayOutputStream bytes=new ByteArrayOutputStream();int c;
        while((c=input.read())!=-1) {
            if(c==10)return bytes.toString("US-ASCII");
            if(c<32||c>126||bytes.size()>=22000)throw new IOException("Invalid bounded ASCII request line");
            bytes.write(c);
        }
        if(bytes.size()!=0)throw new IOException("Partial request line");
        return null;
    }
    static void frame(int sequence,Reply reply)throws IOException {
        if(reply.out.length>LIMIT||reply.err.length>ERROR_LIMIT)throw new IOException("Reply bound exceeded");
        String wire="RF312B "+sequence+" "+reply.code+" "+reply.out.length+" "+reply.err.length+"\nO "
                +Base64.getEncoder().encodeToString(reply.out)+"\nE "+Base64.getEncoder().encodeToString(reply.err)+"\n";
        System.out.print(wire);System.out.flush();
        if(System.out.checkError())throw new IOException("Owned reply stream closed");
    }
    public static void main(String[] args) {
        try {
            if(args.length!=3||!args[0].matches("[a-f0-9]{32}")||!args[2].matches("[1-9][0-9]{4,8}"))throw new IOException("Invalid owned identity");
            int uid=Integer.parseInt(args[2]);String scratch="files/"+args[0]+"/owned-ui";
            if(uid!=Process.myUid()||!scratch.equals(args[1]))throw new IOException("Owner differs");
            File folder=new File(scratch);StructStat info=Os.stat(scratch);
            if(!folder.isDirectory()||Files.isSymbolicLink(folder.toPath())||info.st_uid!=uid||(info.st_mode&0777)!=0700)throw new IOException("Private scratch differs");
            Os.umask(0077);base=new File("files/"+args[0]);
            if(!base.isDirectory()||Files.isSymbolicLink(base.toPath()))throw new IOException("Private namespace differs");
            InputStream input=new BufferedInputStream(System.in);int last=0;String value;
            while((value=line(input))!=null) {
                String[] fields=value.split(" ",-1);
                if(fields.length!=5||!fields[0].equals("RF312B")||!fields[1].matches("[1-9][0-9]{0,4}"))throw new IOException("Invalid fixed request");
                int sequence=Integer.parseInt(fields[1]);String operation=fields[2],name=fields[3],payload=fields[4];
                if(sequence!=last+1||sequence>16384||payload.length()>21848)throw new IOException("Ordinal or payload differs");
                last=sequence;boolean close=operation.equals("close");
                if(!operation.matches("ping|get-stable|exchange|close")||!operation.equals("exchange")&&!payload.equals("-"))throw new IOException("Unsupported UI operation");
                if((operation.equals("ping")||close)&&!name.equals("-"))throw new IOException("Fixed empty name required");
                Reply reply=new Reply();
                try {
                    if(operation.equals("ping"))reply.out="rf312-control-ready\n".getBytes(StandardCharsets.US_ASCII);
                    else if(operation.equals("get-stable"))reply.out=stableRead(name);
                    else if(operation.equals("exchange"))reply.out=exchange(name,payload);
                } catch(Refusal refusal) { reply.code=refusal.code;reply.out=refusal.out;reply.err=refusal.err; }
                catch(IOException error) { reply.code=65;reply.err=error.toString().getBytes(StandardCharsets.UTF_8); }
                frame(sequence,reply);
                if(close) { System.exit(0);return; }
            }
            System.exit(65);
        } catch(Throwable error) {
            System.err.println("RF312 owned UI refused: "+error.getClass().getSimpleName());System.exit(64);
        }
    }
}
