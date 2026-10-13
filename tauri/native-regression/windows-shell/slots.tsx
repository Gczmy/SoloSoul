import { Home, Fingerprint, FileText } from 'lucide-react';
import { NavButton } from '@/components/layout/NavButton';
import { useLocation, useNavigate } from 'react-router-dom';
export function PrimaryNavZone({ sidebarPosition }: { sidebarPosition: 'left' | 'right' }) {
  const navigate = useNavigate();
  const location = useLocation();
  return (
    <>
      {[
        ['/', 'Home', Home],
        ['/identity', 'Identity', Fingerprint],
        ['/notes', 'Notes', FileText],
      ].map(([path, label, Icon]) => (
        <NavButton
          key={String(path)}
          Icon={Icon}
          label={String(label)}
          path={String(path)}
          position={sidebarPosition}
          isActive={location.pathname === path}
          onClick={() => navigate(String(path))}
        />
      ))}
    </>
  );
}
export const SecondaryActionBar = () => null;
export const PairingDialog = () => null;
export const useAndroidGlassSurface = () => {};
export const TopFunctionBar = () => null;
export const MobileBottomNav = () => null;
export const AndroidNavigation = () => null;
