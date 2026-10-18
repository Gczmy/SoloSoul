"""Wait for a docked IME resize to reach both native and virtual WebViews."""
import time
import xml.etree.ElementTree as ET
from native_journey_contract import box,visible

APP='com.solosoul.app'


def viewport_signature(tree,node,probe):
    """Fail closed on stale virtual geometry; retain the exact native input gate."""
    if (node.get('package')!=APP or node.get('class')!='android.widget.EditText'
            or probe.get('imeVisible') is not True or probe.get('imeWindowCount')!=1):
        return None
    parents={child:parent for parent in tree.iter() for child in parent}
    ancestors=[];current=parents.get(node)
    while current is not None:
        if current.get('class')=='android.webkit.WebView':ancestors.append(current)
        current=parents.get(current)
    if len(ancestors)!=2 or any(v.get('package')!=APP or not visible(v) for v in ancestors):
        return None
    document,host=ancestors
    try:
        hl,ht,hr,hb=box(host);dl,dt,dr,db=box(document);tl,tt,tr,tb=box(node)
        ime=probe['imeBounds']
        if not isinstance(ime,list) or len(ime)!=1:return None
        il,it,ir,ib=box(ET.Element('node',bounds=ime[0]))
    except (KeyError,TypeError,ValueError):return None
    hw,hh=hr-hl,hb-ht;dw,dh=dr-dl,db-dt
    if min(hw,hh,dw,dh,ir-il,ib-it)<=0:return None
    if not(il<=hl<hr<=ir and abs(hb-it)<=1 and hl<=tl<tr<=hr and ht<=tt<tb<=hb):return None
    # Virtual WebView dimensions use a slightly different scale. Cross-products
    # compare aspect ratios without rounding the measured coordinates; allow
    # only one pixel of quantization per virtual dimension.
    if abs(dw*hh-dh*hw)>hw+hh:return None
    if abs(dl*hw-hl*dw)>hw+abs(hl) or abs(dt*hw-ht*dw)>hw+abs(ht):return None
    return (host.get('bounds'),document.get('bounds'),tuple(ime))


def unique_input(journey,candidates,deadline,*,allow_scroll=False):
    """Use the original phase deadline, three probes and 350 ms stability gate."""
    stable=None;first=0;observations=0;scrolls=0
    while journey.remaining(deadline)>0:
        journey.snapshot(deadline);found=candidates()
        if len(found)!=1:
            stable=None;observations=0
            if not found and allow_scroll and scrolls<8:
                if journey.scroll(deadline):scrolls+=1
            continue
        node=found[0];journey.focused(node.get('package'))
        probe=journey.observer.inspect(node,'type-public',journey.remaining(deadline))
        geometry=viewport_signature(journey.tree,node,probe)
        if not journey.observer.target_ready(node,probe,'type-public') or geometry is None:
            stable=None;observations=0;continue
        current=(journey.observer.target(node),geometry);now=time.monotonic()
        if current!=stable:stable=current;first=now;observations=1
        else:observations+=1
        if observations>=3 and now-first>=.350:return node
